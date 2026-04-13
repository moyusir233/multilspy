use futures_lite::{FutureExt, future::Boxed};


pub trait RequestMsgHandler: Send + Sync + 'static {
    fn handle_request(
        &self,
        params: Option<serde_json::Value>,
    ) -> Boxed<anyhow::Result<Option<serde_json::Value>>>;
}

impl<T: Send + Sync + 'static, Fut> RequestMsgHandler for T
where
    T: Fn(Option<serde_json::Value>) -> Fut,
    Fut: Future<Output = anyhow::Result<Option<serde_json::Value>>> + Send + 'static,
{
    fn handle_request(
        &self,
        params: Option<serde_json::Value>,
    ) -> Boxed<anyhow::Result<Option<serde_json::Value>>> {
        self(params).boxed()
    }
}

pub trait NotificationMsgHandler: Send + Sync + 'static {
    fn handle_notification(&self, params: Option<serde_json::Value>) -> Boxed<anyhow::Result<()>>;
}

impl<T: Send + Sync + 'static, Fut> NotificationMsgHandler for T
where
    T: Fn(Option<serde_json::Value>) -> Fut,
    Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
{
    fn handle_notification(&self, params: Option<serde_json::Value>) -> Boxed<anyhow::Result<()>> {
        self(params).boxed()
    }
}

/// 对齐multilspy实现中需要注册的handler
pub mod request_handlers {
    use std::sync::Weak;

    use dashmap::DashSet;

    pub async fn register_capability_handler(
        params: Option<serde_json::Value>,
    ) -> anyhow::Result<Option<serde_json::Value>> {
        tracing::debug!("register_capability_handler: {:?}", params);

        Ok(None)
    }

    pub async fn execute_client_command_handler(
        params: Option<serde_json::Value>,
    ) -> anyhow::Result<Option<serde_json::Value>> {
        tracing::debug!("execute_client_command_handler: {:?}", params);

        Ok(None)
    }

    pub async fn create_work_done_progress(
        work_done_progress_set: Weak<DashSet<String>>,
        params: Option<serde_json::Value>,
    ) -> anyhow::Result<Option<serde_json::Value>> {
        tracing::debug!("create_work_done_progress: {:?}", params);

        // 如果work done process set还有效，那么需要在其中保存标识work done process的token
        if let Some(set) = work_done_progress_set.upgrade()
            && let Some(token) = params
                .and_then(|params| Some(params.as_object()?.get("token")?.as_str()?.to_owned()))
        {
            tracing::debug!("save work done progress token in set, token: {:?}", token);
            set.insert(token);
        }

        Ok(None)
    }
}

/// 对齐multilspy实现中需要注册的handler
pub mod notification_handlers {
    use std::sync::Weak;

    use dashmap::DashSet;

    pub async fn lang_status_handler(params: Option<serde_json::Value>) -> anyhow::Result<()> {
        tracing::debug!("lang_status_handler: {:?}", params);

        Ok(())
    }

    pub async fn check_experimental_status(
        params: Option<serde_json::Value>,
    ) -> anyhow::Result<()> {
        tracing::debug!("check_experimental_status: {:?}", params);

        Ok(())
    }

    pub async fn window_log_message(params: Option<serde_json::Value>) -> anyhow::Result<()> {
        tracing::debug!("window_log_message: {:?}", params);

        Ok(())
    }

    pub async fn progress_handler(
        work_done_progress_set: Weak<DashSet<String>>,
        params: Option<serde_json::Value>,
    ) -> anyhow::Result<()> {
        tracing::debug!("progress_handler: {:?}", params);

        // 发现params中包含通知work done progress执行完毕的token时，需要删除work done progress set中的token
        if let Some(set) = work_done_progress_set.upgrade()
            && let Some(token) = params.and_then(|params| {
                let params = params.as_object()?;
                let token = params.get("token")?.as_str()?.to_owned();
                let value = params.get("value")?.as_object()?;

                let is_end = value.get("kind")?.as_str()?.trim().eq("end");

                is_end.then_some(token)
            })
        {
            tracing::debug!(
                "delete work done progress token from set, token: {:?}",
                token
            );
            set.remove(&token);
        }

        Ok(())
    }

    pub async fn do_nothing(params: Option<serde_json::Value>) -> anyhow::Result<()> {
        tracing::debug!("do_nothing: {:?}", params);

        Ok(())
    }
}
