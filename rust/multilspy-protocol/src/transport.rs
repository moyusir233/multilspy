use super::error::ProtocolError;
use super::json_rpc::{Notification, Request, Response};
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub type StdioTransport = Transport<tokio::process::ChildStdout, tokio::process::ChildStdin>;
pub type StdioTransportReader = TransportReader<tokio::process::ChildStdout>;
pub type StdioTransportWriter = TransportWriter<tokio::process::ChildStdin>;

#[derive(derive_more::From, derive_more::TryInto, Debug, Serialize)]
#[serde(untagged)]
pub enum LSPMessage {
    Request(Request),
    Response(Response),
    Notification(Notification),
}

pub trait LSPMessageSender {
    fn send_message(
        &mut self,
        message: LSPMessage,
    ) -> impl Future<Output = Result<(), ProtocolError>> + Send;

    fn send_request(
        &mut self,
        request: Request,
    ) -> impl Future<Output = Result<(), ProtocolError>> + Send {
        self.send_message(request.into())
    }

    fn send_response(
        &mut self,
        response: Response,
    ) -> impl Future<Output = Result<(), ProtocolError>> + Send {
        self.send_message(response.into())
    }

    fn send_notification(
        &mut self,
        notification: Notification,
    ) -> impl Future<Output = Result<(), ProtocolError>> + Send {
        self.send_message(notification.into())
    }
}

pub trait LSPMessageReceiver {
    fn receive_message(&mut self)
    -> impl Future<Output = Result<LSPMessage, ProtocolError>> + Send;
}

#[derive(Debug)]
pub struct TransportReader<R> {
    reader: tokio::io::BufReader<R>,
}

impl<R: AsyncRead + Unpin + Send> LSPMessageReceiver for TransportReader<R> {
    async fn receive_message(&mut self) -> Result<LSPMessage, ProtocolError> {
        let mut line = String::new();
        let mut content_length;

        // Read headers
        loop {
            line.clear();
            let bytes_read = self.reader.read_line(&mut line).await?;

            if bytes_read == 0 {
                return Err(ProtocolError::TransportClosed);
            }

            let trim_line = line.trim();
            if trim_line.is_empty() {
                continue;
            }

            if let Some((key, value)) = trim_line.split_once(':')
                && key.trim().eq_ignore_ascii_case("Content-Length")
            {
                content_length = value.trim().parse::<usize>().map_err(|e| {
                    ProtocolError::InvalidMessage(format!(
                        "Invalid Content-Length header, content length line: {}, err: {}",
                        trim_line, e
                    ))
                })?;

                if content_length == 0 {
                    continue;
                }

                // 读取到了content length，继续读取header部分
                line.clear();
                loop {
                    let bytes_read = self.reader.read_line(&mut line).await?;

                    if bytes_read == 0 {
                        return Err(ProtocolError::TransportClosed);
                    }

                    if line.trim().is_empty() {
                        break;
                    }

                    line.clear();
                }
                // 可以开始读取body
                break;
            }
        }

        const MAX_CONTENT_LENGTH: usize = 10 * 1024 * 1024;
        if content_length > MAX_CONTENT_LENGTH {
            return Err(ProtocolError::InvalidMessage(format!(
                "Content-Length {} exceeds maximum allowed size of {} bytes",
                content_length, MAX_CONTENT_LENGTH
            )));
        }

        // Read content
        let mut content = vec![0u8; content_length];
        self.reader.read_exact(&mut content).await?;

        let json_message: serde_json::Value = serde_json::from_slice(&content)?;
        let json_message_map = json_message.as_object().ok_or_else(|| {
            ProtocolError::InvalidMessage("Invalid JSON message, not an object".to_string())
        })?;

        let message = if json_message_map.contains_key("method") {
            if json_message_map.contains_key("id") {
                LSPMessage::Request(serde_json::from_value(json_message)?)
            } else {
                LSPMessage::Notification(serde_json::from_value(json_message)?)
            }
        } else if json_message_map.contains_key("id") {
            LSPMessage::Response(serde_json::from_value(json_message)?)
        } else {
            return Err(ProtocolError::InvalidMessage(
                "Invalid JSON message, not a request, response or notification".to_string(),
            ));
        };

        Ok(message)
    }
}

#[derive(Debug)]
pub struct TransportWriter<W> {
    writer: W,
}

impl<W: AsyncWrite + Unpin + Send> LSPMessageSender for TransportWriter<W> {
    async fn send_message(&mut self, message: LSPMessage) -> Result<(), ProtocolError> {
        let body = serde_json::to_string(&message)?;
        let content_length = body.len();

        let header = format!(
            "Content-Length: {content_length}\r\nContent-Type: application/vscode-jsonrpc; charset=utf-8\r\n\r\n"
        );

        self.writer.write_all(header.as_bytes()).await?;
        self.writer.write_all(body.as_bytes()).await?;
        self.writer.flush().await?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct Transport<R, W> {
    reader: TransportReader<R>,
    writer: TransportWriter<W>,
}

impl<R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Transport<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: TransportReader {
                reader: tokio::io::BufReader::new(reader),
            },
            writer: TransportWriter { writer },
        }
    }

    pub fn split(self) -> (TransportReader<R>, TransportWriter<W>) {
        (self.reader, self.writer)
    }
}

impl<R: AsyncRead + Unpin + Send, W: Send> LSPMessageReceiver for Transport<R, W> {
    async fn receive_message(&mut self) -> Result<LSPMessage, ProtocolError> {
        self.reader.receive_message().await
    }
}

impl<R: Send, W: AsyncWrite + Unpin + Send> LSPMessageSender for Transport<R, W> {
    async fn send_message(&mut self, message: LSPMessage) -> Result<(), ProtocolError> {
        self.writer.send_message(message).await
    }
}

#[cfg(test)]
mod tests {
    use super::super::json_rpc::*;
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_transport_send_receive() {
        let (client_read, server_write) = tokio::io::duplex(4096);
        let (server_read, client_write) = tokio::io::duplex(4096);

        let mut client_transport = Transport::new(client_read, client_write);
        let mut server_transport = Transport::new(server_read, server_write);

        // Send request from client
        let request = Request::new(
            RequestId::Number(1),
            "test".to_string(),
            Some(json!({"key": "value"})),
        );

        client_transport.send_request(request).await.unwrap();

        // Receive request on server
        let received_request: Request = server_transport
            .receive_message()
            .await
            .unwrap()
            .try_into()
            .unwrap();
        assert_eq!(received_request.id, RequestId::Number(1));
        assert_eq!(received_request.method, "test");

        // Send response from server
        let response = Response::success(RequestId::Number(1), json!({"result": "ok"}));

        server_transport
            .send_message(response.into())
            .await
            .unwrap();

        // Receive response on client
        let received_response: Response = client_transport
            .receive_message()
            .await
            .unwrap()
            .try_into()
            .unwrap();
        assert_eq!(received_response.id, RequestId::Number(1));
        assert!(matches!(
            received_response.result,
            Some(ResponseResult::Result(_))
        ));
    }
}
