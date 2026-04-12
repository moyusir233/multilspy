use super::error::ProtocolError;
use super::json_rpc::{Notification, Request, Response};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug)]
pub struct Transport<R, W> {
    reader: tokio::io::BufReader<R>,
    writer: W,
}

impl<R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Transport<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: tokio::io::BufReader::new(reader),
            writer,
        }
    }

    pub async fn send_request(&mut self, request: &Request) -> Result<(), ProtocolError> {
        self.send_message(request).await
    }

    pub async fn send_notification(
        &mut self,
        notification: &Notification,
    ) -> Result<(), ProtocolError> {
        self.send_message(notification).await
    }

    pub async fn send_response(&mut self, response: &Response) -> Result<(), ProtocolError> {
        self.send_message(response).await
    }

    async fn send_message<T: Serialize>(&mut self, message: &T) -> Result<(), ProtocolError> {
        let body = serde_json::to_string(message)?;
        let content_length = body.len();

        let header = format!(
            "Content-Length: {content_length}\r\nContent-Type: application/vscode-jsonrpc; charset=utf-8\r\n\r\n"
        );

        self.writer.write_all(header.as_bytes()).await?;
        self.writer.write_all(body.as_bytes()).await?;
        self.writer.flush().await?;

        Ok(())
    }

    pub async fn receive_response(&mut self) -> Result<Response, ProtocolError> {
        self.receive_message().await
    }

    pub async fn receive_request(&mut self) -> Result<Request, ProtocolError> {
        self.receive_message().await
    }

    pub async fn receive_notification(&mut self) -> Result<Notification, ProtocolError> {
        self.receive_message().await
    }

    async fn receive_message<T: DeserializeOwned>(&mut self) -> Result<T, ProtocolError> {
        let mut line = String::new();
        let mut content_length = None;

        // Read headers
        loop {
            line.clear();
            let bytes_read = self.reader.read_line(&mut line).await?;

            if bytes_read == 0 {
                return Err(ProtocolError::TransportClosed);
            }

            let line = line.trim();
            if line.is_empty() {
                break;
            }

            if let Some((key, value)) = line.split_once(':')
                && key.trim().eq_ignore_ascii_case("Content-Length")
            {
                content_length = Some(value.trim().parse::<usize>().map_err(|e| {
                    ProtocolError::InvalidMessage(format!("Invalid Content-Length header: {}", e))
                })?);
            }
        }

        let content_length = content_length.ok_or_else(|| {
            ProtocolError::InvalidMessage("Missing Content-Length header".to_string())
        })?;

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

        let message = serde_json::from_slice(&content)?;
        Ok(message)
    }
}

pub type StdioTransport = Transport<tokio::process::ChildStdout, tokio::process::ChildStdin>;

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

        client_transport.send_request(&request).await.unwrap();

        // Receive request on server
        let received_request: Request = server_transport.receive_request().await.unwrap();
        assert_eq!(received_request.id, RequestId::Number(1));
        assert_eq!(received_request.method, "test");

        // Send response from server
        let response = Response::success(RequestId::Number(1), json!({"result": "ok"}));

        server_transport.send_response(&response).await.unwrap();

        // Receive response on client
        let received_response: Response = client_transport.receive_response().await.unwrap();
        assert_eq!(received_response.id, RequestId::Number(1));
        assert!(matches!(
            received_response.result,
            Some(ResponseResult::Result(_))
        ));
    }
}
