use hap_common::{hap_fn, HapError};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
#[allow(dead_code)]
struct SendParams {
    smtp_host: String,
    smtp_port: Option<i32>,
    username: String,
    password: String,
    from: String,
    to: Vec<String>,
    subject: String,
    body: String,
    html: Option<bool>,
    cc: Option<Vec<String>>,
    bcc: Option<Vec<String>>,
    attachments: Option<Vec<String>>,
    use_tls: Option<bool>,
}

hap_fn!(hap_email_send, SendParams, |params| {
    use lettre::{Message, SmtpTransport, Transport};
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::message::{header::ContentType, Mailbox};

    let from_mailbox: Mailbox = params.from.parse()
        .map_err(|e| HapError::invalid_param(format!("invalid from address: {e}")))?;

    let mut builder = Message::builder()
        .from(from_mailbox)
        .subject(&params.subject);

    for addr in &params.to {
        let mb: Mailbox = addr.parse()
            .map_err(|e| HapError::invalid_param(format!("invalid to address: {e}")))?;
        builder = builder.to(mb);
    }

    if let Some(cc_list) = &params.cc {
        for addr in cc_list {
            let mb: Mailbox = addr.parse()
                .map_err(|e| HapError::invalid_param(format!("invalid cc address: {e}")))?;
            builder = builder.cc(mb);
        }
    }

    if let Some(bcc_list) = &params.bcc {
        for addr in bcc_list {
            let mb: Mailbox = addr.parse()
                .map_err(|e| HapError::invalid_param(format!("invalid bcc address: {e}")))?;
            builder = builder.bcc(mb);
        }
    }

    let content_type = if params.html.unwrap_or(false) {
        ContentType::TEXT_HTML
    } else {
        ContentType::TEXT_PLAIN
    };

    let email = builder
        .header(content_type)
        .body(params.body.clone())
        .map_err(|e| HapError::internal(format!("build email failed: {e}")))?;

    let creds = Credentials::new(params.username.clone(), params.password.clone());
    let port = params.smtp_port.unwrap_or(587) as u16;

    let transport = if params.use_tls.unwrap_or(true) {
        SmtpTransport::starttls_relay(&params.smtp_host)
            .map_err(|e| HapError::internal(format!("smtp tls error: {e}")))?
            .port(port)
            .credentials(creds)
            .build()
    } else {
        SmtpTransport::builder_dangerous(&params.smtp_host)
            .port(port)
            .credentials(creds)
            .build()
    };

    transport.send(&email)
        .map_err(|e| HapError::internal(format!("send failed: {e}")))?;

    Ok(json!({ "success": true, "recipients": params.to.len() }))
});

fn connect_imap(host: &str, port: u16, username: &str, password: &str)
    -> Result<imap::Session<rustls::StreamOwned<rustls::ClientConnection, std::net::TcpStream>>, HapError>
{
    use rustls::pki_types::ServerName;
    use std::sync::Arc;

    let root_store = rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = Arc::new(rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth());
    let server_name = ServerName::try_from(host)
        .map_err(|e| HapError::internal(format!("invalid hostname: {e}")))?
        .to_owned();
    let conn = rustls::ClientConnection::new(config, server_name)
        .map_err(|e| HapError::internal(format!("tls init: {e}")))?;
    let tcp = std::net::TcpStream::connect((host, port))
        .map_err(|e| HapError::internal(format!("tcp connect: {e}")))?;
    let tls_stream = rustls::StreamOwned::new(conn, tcp);
    let client = imap::Client::new(tls_stream);
    let session = client.login(username, password)
        .map_err(|e| HapError::internal(format!("imap login: {}", e.0)))?;
    Ok(session)
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct FetchParams {
    host: String,
    port: Option<i32>,
    username: String,
    password: String,
    folder: Option<String>,
    limit: Option<i32>,
    unseen_only: Option<bool>,
    use_tls: Option<bool>,
}

hap_fn!(hap_email_fetch, FetchParams, |params| {
    let port = params.port.unwrap_or(993) as u16;
    let folder = params.folder.as_deref().unwrap_or("INBOX");
    let limit = params.limit.unwrap_or(20).max(1).min(100) as usize;

    let mut session = connect_imap(&params.host, port, &params.username, &params.password)?;

    session.select(folder)
        .map_err(|e| HapError::internal(format!("select folder: {e}")))?;

    let query = if params.unseen_only.unwrap_or(false) {
        "UNSEEN"
    } else {
        "ALL"
    };

    let uids = session.uid_search(query)
        .map_err(|e| HapError::internal(format!("search: {e}")))?;

    let mut uid_list: Vec<u32> = uids.into_iter().collect();
    uid_list.sort_unstable();
    uid_list.reverse();
    uid_list.truncate(limit);

    if uid_list.is_empty() {
        let _ = session.logout();
        return Ok(json!([]));
    }

    let uid_set = uid_list.iter().map(|u| u.to_string()).collect::<Vec<_>>().join(",");
    let messages = session.uid_fetch(&uid_set, "(UID FLAGS ENVELOPE BODYSTRUCTURE RFC822.SIZE)")
        .map_err(|e| HapError::internal(format!("fetch: {e}")))?;

    let mut result = vec![];
    for msg in messages.iter() {
        let uid = msg.uid.unwrap_or(0);
        let size = msg.size.unwrap_or(0);
        let flags: Vec<&str> = msg.flags().iter().map(|f| match f {
            imap::types::Flag::Seen => "\\Seen",
            imap::types::Flag::Answered => "\\Answered",
            imap::types::Flag::Flagged => "\\Flagged",
            imap::types::Flag::Deleted => "\\Deleted",
            imap::types::Flag::Draft => "\\Draft",
            imap::types::Flag::Recent => "\\Recent",
            _ => "\\Unknown",
        }).collect();

        let (subject, from, date) = if let Some(env) = msg.envelope() {
            let subj = env.subject.as_ref()
                .map(|s| String::from_utf8_lossy(s).to_string())
                .unwrap_or_default();
            let from_addr = env.from.as_ref()
                .and_then(|addrs| addrs.first())
                .map(|a| {
                    let mbox = a.mailbox.as_ref().map(|m| String::from_utf8_lossy(m).to_string()).unwrap_or_default();
                    let host = a.host.as_ref().map(|h| String::from_utf8_lossy(h).to_string()).unwrap_or_default();
                    format!("{}@{}", mbox, host)
                })
                .unwrap_or_default();
            let dt = env.date.as_ref()
                .map(|d| String::from_utf8_lossy(d).to_string())
                .unwrap_or_default();
            (subj, from_addr, dt)
        } else {
            (String::new(), String::new(), String::new())
        };

        result.push(json!({
            "uid": uid,
            "subject": subject,
            "from": from,
            "date": date,
            "size": size,
            "flags": flags,
            "read": flags.contains(&"\\Seen"),
        }));
    }

    let _ = session.logout();
    Ok(json!(result))
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct ListFoldersParams {
    host: String,
    port: Option<i32>,
    username: String,
    password: String,
    use_tls: Option<bool>,
}

hap_fn!(hap_email_list_folders, ListFoldersParams, |params| {
    let port = params.port.unwrap_or(993) as u16;

    let mut session = connect_imap(&params.host, port, &params.username, &params.password)?;

    let folders = session.list(None, Some("*"))
        .map_err(|e| HapError::internal(format!("list: {e}")))?;

    let result: Vec<serde_json::Value> = folders.iter().map(|f| {
        json!({
            "name": f.name(),
            "delimiter": f.delimiter().map(|c| c.to_string()),
        })
    }).collect();

    let _ = session.logout();
    Ok(json!(result))
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct MarkReadParams {
    host: String,
    port: Option<i32>,
    username: String,
    password: String,
    message_uid: String,
    folder: Option<String>,
    use_tls: Option<bool>,
}

hap_fn!(hap_email_mark_read, MarkReadParams, |params| {
    let port = params.port.unwrap_or(993) as u16;
    let folder = params.folder.as_deref().unwrap_or("INBOX");

    let mut session = connect_imap(&params.host, port, &params.username, &params.password)?;

    session.select(folder)
        .map_err(|e| HapError::internal(format!("select: {e}")))?;

    session.uid_store(&params.message_uid, "+FLAGS (\\Seen)")
        .map_err(|e| HapError::internal(format!("store flags: {e}")))?;

    let _ = session.logout();
    Ok(json!(true))
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct DeleteParams {
    host: String,
    port: Option<i32>,
    username: String,
    password: String,
    message_uid: String,
    folder: Option<String>,
    use_tls: Option<bool>,
}

hap_fn!(hap_email_delete, DeleteParams, |params| {
    let port = params.port.unwrap_or(993) as u16;
    let folder = params.folder.as_deref().unwrap_or("INBOX");

    let mut session = connect_imap(&params.host, port, &params.username, &params.password)?;

    session.select(folder)
        .map_err(|e| HapError::internal(format!("select: {e}")))?;

    session.uid_store(&params.message_uid, "+FLAGS (\\Deleted)")
        .map_err(|e| HapError::internal(format!("store flags: {e}")))?;
    session.expunge()
        .map_err(|e| HapError::internal(format!("expunge: {e}")))?;

    let _ = session.logout();
    Ok(json!(true))
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct DownloadAttachmentParams {
    host: String,
    port: Option<i32>,
    username: String,
    password: String,
    message_uid: String,
    attachment_index: i32,
    save_path: String,
    folder: Option<String>,
    use_tls: Option<bool>,
}

hap_fn!(hap_email_download_attachment, DownloadAttachmentParams, |params| {
    let port = params.port.unwrap_or(993) as u16;
    let folder = params.folder.as_deref().unwrap_or("INBOX");

    let mut session = connect_imap(&params.host, port, &params.username, &params.password)?;

    session.select(folder)
        .map_err(|e| HapError::internal(format!("select: {e}")))?;

    let messages = session.uid_fetch(&params.message_uid, "BODY[]")
        .map_err(|e| HapError::internal(format!("fetch body: {e}")))?;

    let msg = messages.iter().next()
        .ok_or_else(|| HapError::internal("message not found"))?;

    let body = msg.body()
        .ok_or_else(|| HapError::internal("message has no body"))?;

    use mail_parser::MimeHeaders;
    let parsed = mail_parser::MessageParser::default()
        .parse(body)
        .ok_or_else(|| HapError::internal("failed to parse email"))?;

    let idx = params.attachment_index as usize;
    let mut att_count = 0usize;
    for part in parsed.parts.iter() {
        if part.attachment_name().is_some() || part.content_disposition().map(|d| d.ctype() == "attachment").unwrap_or(false) {
            if att_count == idx {
                let content = part.contents();
                std::fs::write(&params.save_path, content)
                    .map_err(|e| HapError::internal(format!("write file: {e}")))?;
                let filename = part.attachment_name().unwrap_or("attachment");
                let _ = session.logout();
                return Ok(json!({
                    "path": params.save_path,
                    "filename": filename,
                    "size": content.len(),
                    "success": true
                }));
            }
            att_count += 1;
        }
    }

    let _ = session.logout();
    Err(HapError::invalid_param(format!("attachment index {} not found, total: {}", idx, att_count)))
});
