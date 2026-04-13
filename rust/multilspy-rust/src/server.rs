use crate::server::handlers::{NotificationMsgHandler, RequestMsgHandler};

use super::config::RustAnalyzerConfig;
use super::error::ServerError;
use dashmap::{DashMap, DashSet};
use fluent_uri::Uri;
use fluent_uri::component::Scheme;
use fluent_uri::pct_enc::EStr;
use multilspy_protocol::error::ErrorCodes;
use multilspy_protocol::json_rpc::{Notification, Request, RequestId, Response, ResponseResult};
use multilspy_protocol::protocol::common::WorkspaceFolder;
use multilspy_protocol::protocol::requests::InitializeParams;
use multilspy_protocol::transport::{
    LSPMessageReceiver, LSPMessageSender, StdioTransport, StdioTransportReader,
    StdioTransportWriter, Transport,
};
use std::fmt::Debug;
use std::process::{Command, Stdio};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::process::Child;

pub(super) mod handlers;

enum MessageData {
    Request {
        method: String,
        params: Option<serde_json::Value>,
        /// 用户发送response处理结果的channel tx
        tx: tokio::sync::oneshot::Sender<anyhow::Result<Option<serde_json::Value>>>,
    },
    Notification {
        method: String,
        params: Option<serde_json::Value>,
    },
    Response {
        id: RequestId,
        result: anyhow::Result<Option<serde_json::Value>>,
        /// 指定发送给server error resp时使用的错误码
        error_code: Option<ErrorCodes>,
    },
}

impl Debug for MessageData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Request {
                method,
                params,
                tx: _,
            } => f
                .debug_struct("Request")
                .field("method", method)
                .field("params", params)
                .finish(),
            Self::Notification { method, params } => f
                .debug_struct("Notification")
                .field("method", method)
                .field("params", params)
                .finish(),
            Self::Response {
                id,
                result,
                error_code,
            } => f
                .debug_struct("Response")
                .field("id", id)
                .field("result", result)
                .field("error_code", error_code)
                .finish(),
        }
    }
}

/// server管理着lsp server process，并封装lsp message的发送与接收逻辑
/// - 对于client需要发送的request，会将request发送到lsp server process，并等待response，然后通过oneshot channel将response的结果返回给上游
/// - 对于lsp server发送的request，会调用对应的handler来处理(以request id为key，保存在`response_handlers`中)，并基于handler的处理结果
/// - 对于lsp server发送的notification，会调用对应的handler来处理(以method为key，保存在`notification_handlers`中)
pub struct RustAnalyzerServer {
    child: tokio::sync::Mutex<Child>,
    need_send_msg_tx: tokio::sync::mpsc::Sender<MessageData>,
    /// 以request id为key，保存发送请求结果的channel tx，处理lsp server lsp response msg时，
    /// 从map中获取对应的channel tx，将结果发送到tx中
    response_handlers:
        DashMap<u64, tokio::sync::oneshot::Sender<anyhow::Result<Option<serde_json::Value>>>>,
    /// 以method为key，当服务端发送相应method request时调用对应的handler来处理
    request_handlers: DashMap<String, Arc<Box<dyn RequestMsgHandler>>>,
    /// 以method为key，当服务端发送相应method notification时调用对应的handler来处理
    notification_handlers: DashMap<String, Arc<Box<dyn NotificationMsgHandler>>>,
}

fn spawn_lsp_server_process(
    config: &RustAnalyzerConfig,
) -> Result<(Child, StdioTransport), ServerError> {
    tracing::info!("spawn lsp server process");

    // Spawn rust-analyzer process
    let mut cmd = Command::new(&config.server_executable_path);

    // Set working directory to project root
    cmd.current_dir(&config.project_root);

    // Add environment variables
    for (key, value) in &config.env {
        cmd.env(key, value);
    }

    cmd.stdout(Stdio::piped()).stdin(Stdio::piped());

    if let Some(path) = &config.ra_stderr_log_path
        && path.exists()
        && path.is_file()
    {
        cmd.stderr(std::fs::File::create(path)?);
    } else {
        cmd.stderr(Stdio::piped());
    }

    let mut child = tokio::process::Command::from(cmd)
        .spawn()
        .map_err(|e| anyhow::anyhow!("spawn lsp server process failed: {:?}", e))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ServerError::IoError(std::io::Error::other("Failed to get stdout")))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| ServerError::IoError(std::io::Error::other("Failed to get stdin")))?;

    let transport = Transport::new(stdout, stdin);

    Ok((child, transport))
}

/// 处理接收到的response，通过channel将resp结果转发给上游调用方
#[tracing::instrument(level = "trace", ret, skip(resp_result_tx))]
async fn handle_response_handler(
    resp_result_tx: tokio::sync::oneshot::Sender<anyhow::Result<Option<serde_json::Value>>>,
    response: Response,
) -> anyhow::Result<()> {
    match response.result {
        Some(ResponseResult::Result(value)) => {
            resp_result_tx
                .send(Ok(Some(value)))
                .map_err(|e| anyhow::anyhow!("failed to send resp result by channel: {:?}", e))?;
        }
        Some(ResponseResult::Error(err)) => {
            resp_result_tx
                .send(Err(anyhow::anyhow!(
                    "receive error from lsp server: {:?}",
                    err
                )))
                .map_err(|e| anyhow::anyhow!("failed to send resp result by channel: {:?}", e))?;
        }
        None => {
            // 无结果，认为请求成功
            resp_result_tx
                .send(Ok(None))
                .map_err(|e| anyhow::anyhow!("failed to send resp result by channel: {:?}", e))?;
        }
    }

    Ok(())
}

/// 处理接收到的request
#[tracing::instrument(level = "trace", ret, skip(server, request_handler), ret)]
async fn handle_request_handler(
    server: Arc<RustAnalyzerServer>,
    request_handler: Option<Arc<Box<dyn RequestMsgHandler>>>,
    request: Request,
) -> anyhow::Result<()> {
    // 如果不存在对应的handler，那么也需要通知服务端error的resp

    let (result, error_code) = match request_handler {
        Some(request_handler) => {
            let result = request_handler.handle_request(request.params).await;
            (result, Some(ErrorCodes::InternalError))
        }
        None => {
            tracing::warn!("request handler not found for method: {}", request.method);
            (
                Err(anyhow::anyhow!(
                    "request handler not found for method: {}",
                    request.method
                )),
                Some(ErrorCodes::MethodNotFound),
            )
        }
    };

    server
        .send_message(MessageData::Response {
            id: request.id,
            result,
            error_code,
        })
        .await?;

    Ok(())
}

/// 处理接收到的notification
#[tracing::instrument(level = "trace", ret, skip(notification_handler), ret)]
async fn handle_notification_handler(
    notification_handler: Arc<Box<dyn NotificationMsgHandler>>,
    notification: Notification,
) -> anyhow::Result<()> {
    // notification不需要resp，仅直接调用handler
    notification_handler
        .handle_notification(notification.params)
        .await?;

    Ok(())
}

async fn run_receive_msg_task(
    server: Weak<RustAnalyzerServer>,
    mut transport: StdioTransportReader,
) -> anyhow::Result<()> {
    tracing::info!("run receive msg task");

    loop {
        let msg = transport.receive_message().await?;

        let Some(server) = server.upgrade() else {
            tracing::debug!("server was dropped, finish");
            break;
        };

        tracing::trace!("receive lsp message: {:?}", msg);

        match msg {
            multilspy_protocol::transport::LSPMessage::Request(request) => {
                let request_handler = server.request_handlers.get(&request.method);

                tokio::spawn(handle_request_handler(
                    server.clone(),
                    request_handler.map(|pair| pair.value().clone()),
                    request,
                ));
            }
            multilspy_protocol::transport::LSPMessage::Response(response) => {
                let request_id = match &response.id {
                    RequestId::Number(id) => *id,
                    RequestId::String(str_id) => str_id.parse()?,
                };
                let Some((_, tx)) = server.response_handlers.remove(&request_id) else {
                    tracing::warn!("response handler not found for request id: {}", request_id);
                    continue;
                };

                tokio::spawn(handle_response_handler(tx, response));
            }
            multilspy_protocol::transport::LSPMessage::Notification(notification) => {
                let Some(notification_handler) =
                    server.notification_handlers.get(&notification.method)
                else {
                    tracing::warn!(
                        "notification handler not found for method: {}",
                        notification.method
                    );
                    continue;
                };

                tokio::spawn(handle_notification_handler(
                    notification_handler.value().clone(),
                    notification,
                ));
            }
        }
    }

    Ok(())
}

async fn run_send_msg_task(
    server: Weak<RustAnalyzerServer>,
    mut rx: tokio::sync::mpsc::Receiver<MessageData>,
    mut transport: StdioTransportWriter,
) -> anyhow::Result<()> {
    tracing::info!("run send msg task");

    let mut request_id = 0;
    while let Some(msg) = rx.recv().await {
        let Some(server) = server.upgrade() else {
            tracing::debug!("server was dropped, finish");
            break;
        };

        request_id += 1;
        tracing::trace!("send lsp message: {:?}, request id: {}", msg, request_id);

        match msg {
            MessageData::Request { method, params, tx } => {
                // 接收到了request，提前注册好处理response的handler
                server.register_response_handler(request_id, tx);

                transport
                    .send_request(Request::new(RequestId::Number(request_id), method, params))
                    .await?;
            }
            MessageData::Notification { method, params } => {
                transport
                    .send_notification(Notification::new(method, params))
                    .await?;
            }
            MessageData::Response {
                id,
                result,
                error_code,
            } => match result {
                Ok(resp_result) => {
                    transport
                        .send_response(Response::success(
                            id,
                            resp_result.unwrap_or(serde_json::Value::Null),
                        ))
                        .await?;
                }
                Err(err) => {
                    transport
                        .send_response(Response::error(
                            id,
                            error_code.unwrap_or(ErrorCodes::InternalError),
                            err.to_string(),
                            None,
                        ))
                        .await?;
                }
            },
        }
    }

    Ok(())
}

/// 轮询检查lsp server work done progress set是否为空，若为空则通知server_ready
async fn run_check_work_done_progress_set_task(
    work_done_progress_set: Arc<DashSet<String>>,
    server_ready: Arc<tokio::sync::Notify>,
    num_epoch: u32,
    initial_sleep_time: Duration,
    sleep_time_per_epoch: Duration,
) -> anyhow::Result<()> {
    tracing::info!("run check work done progress set task");

    tokio::time::sleep(initial_sleep_time).await;

    for _ in 0..num_epoch {
        if work_done_progress_set.is_empty() {
            server_ready.notify_waiters();
            break;
        }

        tokio::time::sleep(sleep_time_per_epoch).await;
    }

    tracing::warn!("work done progress set still is not empty, but force notify server_ready");

    Ok(())
}

impl RustAnalyzerServer {
    fn register_request_handler<T: RequestMsgHandler>(&self, method: String, handler: T) {
        self.request_handlers
            .insert(method, Arc::new(Box::new(handler)));
    }

    fn register_notification_handler<T: NotificationMsgHandler>(&self, method: String, handler: T) {
        self.notification_handlers
            .insert(method, Arc::new(Box::new(handler)));
    }

    fn register_response_handler(
        &self,
        request_id: u64,
        tx: tokio::sync::oneshot::Sender<anyhow::Result<Option<serde_json::Value>>>,
    ) {
        self.response_handlers.insert(request_id, tx);
    }

    async fn register_handlers(
        &self,
        work_done_progress_set: Weak<DashSet<String>>,
    ) -> anyhow::Result<()> {
        self.register_request_handler(
            "client/registerCapability".to_string(),
            handlers::request_handlers::register_capability_handler,
        );
        {
            let work_done_progress_set = work_done_progress_set.clone();
            self.register_request_handler(
                "window/workDoneProgress/create".to_string(),
                move |params: Option<serde_json::Value>| {
                    let work_done_progress_set = work_done_progress_set.clone();
                    async move {
                        handlers::request_handlers::create_work_done_progress(
                            work_done_progress_set,
                            params,
                        )
                        .await
                    }
                },
            );
        }
        self.register_request_handler(
            "workspace/executeClientCommand".to_string(),
            handlers::request_handlers::execute_client_command_handler,
        );

        self.register_notification_handler(
            "language/status".to_string(),
            handlers::notification_handlers::lang_status_handler,
        );
        self.register_notification_handler(
            "window/logMessage".to_string(),
            handlers::notification_handlers::window_log_message,
        );
        self.register_notification_handler(
            "$/progress".to_string(),
            move |params: Option<serde_json::Value>| {
                let work_done_progress_set = work_done_progress_set.clone();
                async move {
                    handlers::notification_handlers::progress_handler(
                        work_done_progress_set,
                        params,
                    )
                    .await
                }
            },
        );
        self.register_notification_handler(
            "textDocument/publishDiagnostics".to_string(),
            handlers::notification_handlers::do_nothing,
        );
        self.register_notification_handler(
            "language/actionableNotification".to_string(),
            handlers::notification_handlers::do_nothing,
        );
        self.register_notification_handler(
            "experimental/serverStatus".to_string(),
            handlers::notification_handlers::check_experimental_status,
        );

        Ok(())
    }

    async fn initialize(&self, config: RustAnalyzerConfig) -> Result<(), ServerError> {
        // 正式开始请求初始化前，先注册完毕必要的handler，以及启动检查work done progress set是否为空的task
        let work_done_progress_set = Arc::new(DashSet::new());
        let server_ready = Arc::new(tokio::sync::Notify::new());

        self.register_handlers(Arc::downgrade(&work_done_progress_set))
            .await?;

        // 最长等待lsp server ready的时间设置为20分钟
        tokio::spawn(run_check_work_done_progress_set_task(
            work_done_progress_set,
            server_ready.clone(),
            40,
            config.wait_work_done_progress_create_max_time,
            Duration::from_secs(30),
        ));

        tracing::info!("start initialize rust analyzer lsp server");

        let mut params: InitializeParams = serde_json::from_str(
            &tokio::fs::read_to_string(&config.initialize_params_path).await?,
        )?;

        // 填充初始化参数中必要的字段
        params.process_id = Some(std::process::id());

        let absolute_project_root = config.project_root.canonicalize()?;
        let project_name = absolute_project_root
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let root_uri = Uri::builder()
            .scheme(Scheme::new("file").ok_or_else(|| {
                ServerError::InitializationFailed(
                    "failed to build root_uri of `InitializeParams`".to_string(),
                )
            })?)
            .path(
                absolute_project_root
                    .to_str()
                    .and_then(|root_path| EStr::new(root_path))
                    .ok_or_else(|| {
                        ServerError::InitializationFailed(
                            "failed to build root_uri of `InitializeParams`".to_string(),
                        )
                    })?,
            )
            .build()
            .map_err(|e| ServerError::InitializationFailed(e.to_string()))?
            .to_string();

        params.root_uri = Some(root_uri.clone());
        params.workspace_folders = Some(vec![WorkspaceFolder {
            uri: root_uri,
            name: project_name,
        }]);

        tracing::info!("initialize params: {:#?}", params);

        let resp = self
            .send_request("initialize".to_string(), Some(params))
            .await?;
        tracing::debug!("resp of initialize server: {:#?}", resp);

        tracing::info!("send initialized notification");
        self.send_notification::<()>("initialized".to_string(), None)
            .await?;

        tracing::info!("wait for server ready");

        // 等待lsp server ready
        server_ready.notified().await;

        tracing::info!("finish wait for server ready");

        Ok(())
    }

    async fn send_message(&self, message: MessageData) -> anyhow::Result<()> {
        self.need_send_msg_tx.send(message).await?;

        Ok(())
    }
}

impl RustAnalyzerServer {
    pub async fn start_server(config: RustAnalyzerConfig) -> Result<Arc<Self>, ServerError> {
        tracing::info!("start start rust analyzer lsp server, config: {:?}", config);

        let (child, transport) = spawn_lsp_server_process(&config)?;
        let (request_tx, request_rx) = tokio::sync::mpsc::channel::<MessageData>(256);

        let server = Arc::new(Self {
            child: tokio::sync::Mutex::new(child),
            need_send_msg_tx: request_tx,
            response_handlers: DashMap::new(),
            request_handlers: DashMap::new(),
            notification_handlers: DashMap::new(),
        });
        let (reader, writer) = transport.split();

        // 启动处理lsp server的消息发送与接收的task
        tokio::spawn(run_receive_msg_task(Arc::downgrade(&server), reader));
        tokio::spawn(run_send_msg_task(
            Arc::downgrade(&server),
            request_rx,
            writer,
        ));

        // Initialize server
        server.initialize(config).await?;

        Ok(server)
    }

    pub async fn shutdown(self: Arc<Self>) -> Result<(), ServerError> {
        tracing::info!("start shutdown rust analyzer lsp server");

        // 先通知lsp server shutdown
        self.send_request::<()>("shutdown".to_string(), None)
            .await?;

        // kill掉子进程
        self.child.lock().await.start_kill()?;

        tracing::info!("finish shutdown rust analyzer lsp server");

        Ok(())
    }

    pub async fn send_request<T: serde::Serialize>(
        &self,
        method: String,
        params: Option<T>,
    ) -> Result<Option<serde_json::Value>, ServerError> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        self.send_message(MessageData::Request {
            method,
            params: params.and_then(|p| serde_json::to_value(p).ok()),
            tx,
        })
        .await?;

        let resp_result = rx
            .await
            .map_err(|err| anyhow::anyhow!("failed to receive resp result: {:?}", err))??;

        Ok(resp_result)
    }

    pub async fn send_notification<T: serde::Serialize>(
        &self,
        method: String,
        params: Option<T>,
    ) -> Result<(), ServerError> {
        self.send_message(MessageData::Notification {
            method,
            params: params.and_then(|p| serde_json::to_value(p).ok()),
        })
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tracing_subscriber::EnvFilter;

    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_server_lifecycle() {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .init();

        // Skip test if rust-analyzer is not installed
        if Command::new("rust-analyzer")
            .arg("--version")
            .output()
            .is_err()
        {
            println!("rust-analyzer not installed, skipping test");
            return;
        }

        let config = RustAnalyzerConfig::new(
            PathBuf::from("./test-rust-project"),
            PathBuf::from("./ra_initialize_params.json"),
        )
        .with_stderr_log_path(PathBuf::from("./ra_stderr.log"))
        .with_env("RA_LOG".to_string(), "info".to_string())
        .with_wait_work_done_progress_create_max_time(Duration::from_secs(5));

        let server = RustAnalyzerServer::start_server(config).await.unwrap();

        server.shutdown().await.unwrap();
    }
}
