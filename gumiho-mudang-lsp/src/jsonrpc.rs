use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    pub id: i64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    pub id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

#[derive(Debug, Clone)]
pub enum IncomingMessage {
    Request(Request),
    Notification(Notification),
    Response(Response),
}

#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("missing Content-Length header")]
    MissingContentLength,
    #[error("invalid Content-Length: {0}")]
    InvalidContentLength(String),
    #[error("invalid json: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("connection closed")]
    ConnectionClosed,
}

pub async fn read_message(
    reader: &mut (impl AsyncBufRead + Unpin),
) -> Result<IncomingMessage, CodecError> {
    let mut content_length: Option<usize> = None;

    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            return Err(CodecError::ConnectionClosed);
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }

        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| CodecError::InvalidContentLength(value.trim().to_string()))?,
            );
        }
    }

    let length = content_length.ok_or(CodecError::MissingContentLength)?;
    let mut body = vec![0u8; length];
    tokio::io::AsyncReadExt::read_exact(reader, &mut body).await?;

    let raw: Value = serde_json::from_slice(&body)?;

    if raw.get("id").is_some() && raw.get("method").is_some() {
        Ok(IncomingMessage::Request(serde_json::from_value(raw)?))
    } else if raw.get("id").is_some() {
        Ok(IncomingMessage::Response(serde_json::from_value(raw)?))
    } else {
        Ok(IncomingMessage::Notification(serde_json::from_value(raw)?))
    }
}

pub async fn write_message(
    writer: &mut (impl AsyncWrite + Unpin),
    body: &[u8],
) -> Result<(), CodecError> {
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(body).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn send_request(
    writer: &mut (impl AsyncWrite + Unpin),
    id: i64,
    method: &str,
    params: Option<Value>,
) -> Result<(), CodecError> {
    let req = Request {
        jsonrpc: "2.0".into(),
        id,
        method: method.into(),
        params,
    };
    let body = serde_json::to_vec(&req)?;
    write_message(writer, &body).await
}

pub async fn send_notification(
    writer: &mut (impl AsyncWrite + Unpin),
    method: &str,
    params: Option<Value>,
) -> Result<(), CodecError> {
    let notif = Notification {
        jsonrpc: "2.0".into(),
        method: method.into(),
        params,
    };
    let body = serde_json::to_vec(&notif)?;
    write_message(writer, &body).await
}

pub fn make_response(id: i64, result: Value) -> Response {
    Response {
        jsonrpc: "2.0".into(),
        id: Some(id),
        result: Some(result),
        error: None,
    }
}

pub fn make_error_response(id: i64, code: i64, message: String) -> Response {
    Response {
        jsonrpc: "2.0".into(),
        id: Some(id),
        result: None,
        error: Some(RpcError {
            code,
            message,
            data: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tokio::io::BufReader;

    fn make_lsp_message(json: &str) -> Vec<u8> {
        format!("Content-Length: {}\r\n\r\n{}", json.len(), json).into_bytes()
    }

    #[tokio::test]
    async fn test_parse_request() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let data = make_lsp_message(json);
        let mut reader = BufReader::new(Cursor::new(data));
        let msg = read_message(&mut reader).await.unwrap();
        match msg {
            IncomingMessage::Request(req) => {
                assert_eq!(req.id, 1);
                assert_eq!(req.method, "initialize");
            }
            _ => panic!("expected request"),
        }
    }

    #[tokio::test]
    async fn test_parse_notification() {
        let json = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":"file:///test.rs"}}"#;
        let data = make_lsp_message(json);
        let mut reader = BufReader::new(Cursor::new(data));
        let msg = read_message(&mut reader).await.unwrap();
        match msg {
            IncomingMessage::Notification(notif) => {
                assert_eq!(notif.method, "textDocument/publishDiagnostics");
            }
            _ => panic!("expected notification"),
        }
    }

    #[tokio::test]
    async fn test_parse_response() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}"#;
        let data = make_lsp_message(json);
        let mut reader = BufReader::new(Cursor::new(data));
        let msg = read_message(&mut reader).await.unwrap();
        match msg {
            IncomingMessage::Response(resp) => {
                assert_eq!(resp.id, Some(1));
                assert!(resp.result.is_some());
            }
            _ => panic!("expected response"),
        }
    }

    #[tokio::test]
    async fn test_write_roundtrip() {
        let mut buf = Vec::new();
        send_request(
            &mut buf,
            42,
            "test/method",
            Some(serde_json::json!({"key": "val"})),
        )
        .await
        .unwrap();

        let mut reader = BufReader::new(Cursor::new(buf));
        let msg = read_message(&mut reader).await.unwrap();
        match msg {
            IncomingMessage::Request(req) => {
                assert_eq!(req.id, 42);
                assert_eq!(req.method, "test/method");
            }
            _ => panic!("expected request"),
        }
    }

    #[tokio::test]
    async fn test_missing_content_length() {
        let data = b"\r\n{\"jsonrpc\":\"2.0\"}";
        let mut reader = BufReader::new(Cursor::new(data.to_vec()));
        let err = read_message(&mut reader).await.unwrap_err();
        assert!(matches!(err, CodecError::MissingContentLength));
    }

    #[tokio::test]
    async fn test_invalid_json() {
        let data = make_lsp_message("not json");
        let mut reader = BufReader::new(Cursor::new(data));
        let err = read_message(&mut reader).await.unwrap_err();
        assert!(matches!(err, CodecError::InvalidJson(_)));
    }

    #[tokio::test]
    async fn test_connection_closed() {
        let mut reader = BufReader::new(Cursor::new(Vec::<u8>::new()));
        let err = read_message(&mut reader).await.unwrap_err();
        assert!(matches!(err, CodecError::ConnectionClosed));
    }
}
