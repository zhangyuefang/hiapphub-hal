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

#[derive(Deserialize)]
#[allow(dead_code)]
struct FetchParams {
    protocol: String,
    host: String,
    port: Option<i32>,
    username: String,
    password: String,
    folder: Option<String>,
    limit: Option<i32>,
    unseen_only: Option<bool>,
    use_tls: Option<bool>,
}

hap_fn!(hap_email_fetch, FetchParams, |_params| {
    Ok(json!([]))
});

#[derive(Deserialize)]
#[allow(dead_code)]
struct ListFoldersParams {
    protocol: String,
    host: String,
    port: Option<i32>,
    username: String,
    password: String,
    use_tls: Option<bool>,
}

hap_fn!(hap_email_list_folders, ListFoldersParams, |_params| {
    Ok(json!([]))
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

hap_fn!(hap_email_mark_read, MarkReadParams, |_params| {
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

hap_fn!(hap_email_delete, DeleteParams, |_params| {
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
    use_tls: Option<bool>,
}

hap_fn!(hap_email_download_attachment, DownloadAttachmentParams, |params| {
    Ok(json!({ "path": params.save_path, "success": true }))
});
