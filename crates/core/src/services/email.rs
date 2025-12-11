//! Email notification service.

use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, MultiPart, SinglePart, header::ContentType},
    transport::smtp::authentication::Credentials,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use misskey_common::{AppError, AppResult};

/// Email provider configuration.
#[derive(Debug, Clone)]
pub enum EmailProvider {
    /// SMTP configuration
    Smtp(SmtpConfig),
    /// Amazon SES
    Ses(SesConfig),
    /// `SendGrid`
    SendGrid(SendGridConfig),
    /// Mailgun
    Mailgun(MailgunConfig),
}

/// SMTP configuration.
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    /// SMTP host
    pub host: String,
    /// SMTP port
    pub port: u16,
    /// Use TLS
    pub use_tls: bool,
    /// Username
    pub username: Option<String>,
    /// Password
    pub password: Option<String>,
}

/// Amazon SES configuration.
#[derive(Debug, Clone)]
pub struct SesConfig {
    /// AWS region
    pub region: String,
    /// AWS access key ID
    pub access_key_id: String,
    /// AWS secret access key
    pub secret_access_key: String,
}

/// `SendGrid` configuration.
#[derive(Debug, Clone)]
pub struct SendGridConfig {
    /// `SendGrid` API key
    pub api_key: String,
}

/// Mailgun configuration.
#[derive(Debug, Clone)]
pub struct MailgunConfig {
    /// Mailgun API key
    pub api_key: String,
    /// Mailgun domain
    pub domain: String,
    /// Use EU region
    pub eu_region: bool,
}

/// Email configuration.
#[derive(Debug, Clone)]
pub struct EmailConfig {
    /// Email provider
    pub provider: EmailProvider,
    /// From address
    pub from_address: String,
    /// From name
    pub from_name: String,
    /// Reply-to address (optional)
    pub reply_to: Option<String>,
    /// Instance name (for templates)
    pub instance_name: String,
    /// Instance URL (for templates)
    pub instance_url: String,
}

/// Email notification types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EmailNotificationType {
    /// New follower
    Follow,
    /// New mention
    Mention,
    /// Direct message
    Message,
    /// Password reset
    PasswordReset,
    /// Email verification
    EmailVerification,
    /// Weekly digest
    WeeklyDigest,
    /// Monthly digest
    MonthlyDigest,
    /// Account security alert
    SecurityAlert,
    /// Welcome email
    Welcome,
}

impl std::fmt::Display for EmailNotificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Follow => "follow",
            Self::Mention => "mention",
            Self::Message => "message",
            Self::PasswordReset => "password_reset",
            Self::EmailVerification => "email_verification",
            Self::WeeklyDigest => "weekly_digest",
            Self::MonthlyDigest => "monthly_digest",
            Self::SecurityAlert => "security_alert",
            Self::Welcome => "welcome",
        };
        write!(f, "{s}")
    }
}

/// Email message to be sent.
#[derive(Debug)]
pub struct EmailMessage {
    /// Recipient email address
    pub to: String,
    /// Subject line
    pub subject: String,
    /// Plain text body
    pub text_body: String,
    /// HTML body (optional)
    pub html_body: Option<String>,
    /// Reply-to address (optional, overrides config)
    pub reply_to: Option<String>,
    /// Custom headers
    pub headers: HashMap<String, String>,
}

/// Template variables for emails.
#[derive(Debug, Default)]
pub struct EmailTemplateVars {
    /// User's display name
    pub user_name: Option<String>,
    /// User's username
    pub username: Option<String>,
    /// Actor's display name (for notifications)
    pub actor_name: Option<String>,
    /// Actor's username (for notifications)
    pub actor_username: Option<String>,
    /// Note text (for mentions/replies)
    pub note_text: Option<String>,
    /// Note URL
    pub note_url: Option<String>,
    /// Reset/verification URL
    pub action_url: Option<String>,
    /// Action code/token
    pub action_code: Option<String>,
    /// Digest items count
    pub item_count: Option<u32>,
    /// Additional custom variables
    pub custom: HashMap<String, String>,
}

/// Email delivery result.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailDeliveryResult {
    /// Whether the email was sent successfully
    pub success: bool,
    /// Message ID from provider (if available)
    pub message_id: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Email service.
#[derive(Clone)]
pub struct EmailService {
    config: Option<EmailConfig>,
    http_client: reqwest::Client,
}

impl EmailService {
    /// Create a new email service.
    #[must_use]
    pub fn new(config: Option<EmailConfig>) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
        }
    }

    /// Check if email service is enabled.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.config.is_some()
    }

    /// Send an email.
    pub async fn send(&self, message: EmailMessage) -> AppResult<EmailDeliveryResult> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| AppError::BadRequest("Email service not configured".to_string()))?;

        match &config.provider {
            EmailProvider::Smtp(smtp) => self.send_smtp(smtp, config, message).await,
            EmailProvider::Ses(ses) => self.send_ses(ses, config, message).await,
            EmailProvider::SendGrid(sg) => self.send_sendgrid(sg, config, message).await,
            EmailProvider::Mailgun(mg) => self.send_mailgun(mg, config, message).await,
        }
    }

    /// Send a notification email.
    pub async fn send_notification(
        &self,
        notification_type: EmailNotificationType,
        to: &str,
        vars: EmailTemplateVars,
    ) -> AppResult<EmailDeliveryResult> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| AppError::BadRequest("Email service not configured".to_string()))?;

        let (subject, text_body, html_body) =
            self.render_template(notification_type, &vars, config)?;

        let message = EmailMessage {
            to: to.to_string(),
            subject,
            text_body,
            html_body: Some(html_body),
            reply_to: None,
            headers: HashMap::new(),
        };

        self.send(message).await
    }

    /// Render an email template.
    fn render_template(
        &self,
        notification_type: EmailNotificationType,
        vars: &EmailTemplateVars,
        config: &EmailConfig,
    ) -> AppResult<(String, String, String)> {
        let (subject, text, html) = match notification_type {
            EmailNotificationType::Follow => {
                let actor = vars.actor_name.as_deref().unwrap_or("Someone");
                let subject = format!("{} followed you on {}", actor, config.instance_name);
                let text = format!(
                    "{} (@{}) is now following you on {}.\n\nView their profile: {}/users/{}",
                    actor,
                    vars.actor_username.as_deref().unwrap_or("unknown"),
                    config.instance_name,
                    config.instance_url,
                    vars.actor_username.as_deref().unwrap_or("unknown")
                );
                let html = self.wrap_html(
                    &format!(
                        "<p><strong>{}</strong> (@{}) is now following you on {}.</p>\
                    <p><a href=\"{}/users/{}\">View their profile</a></p>",
                        actor,
                        vars.actor_username.as_deref().unwrap_or("unknown"),
                        config.instance_name,
                        config.instance_url,
                        vars.actor_username.as_deref().unwrap_or("unknown")
                    ),
                    config,
                );
                (subject, text, html)
            }

            EmailNotificationType::Mention => {
                let actor = vars.actor_name.as_deref().unwrap_or("Someone");
                let subject = format!("{} mentioned you on {}", actor, config.instance_name);
                let text = format!(
                    "{} mentioned you:\n\n{}\n\nView the post: {}",
                    actor,
                    vars.note_text.as_deref().unwrap_or(""),
                    vars.note_url.as_deref().unwrap_or("")
                );
                let html = self.wrap_html(
                    &format!(
                        "<p><strong>{}</strong> mentioned you:</p>\
                    <blockquote>{}</blockquote>\
                    <p><a href=\"{}\">View the post</a></p>",
                        actor,
                        vars.note_text.as_deref().unwrap_or(""),
                        vars.note_url.as_deref().unwrap_or("")
                    ),
                    config,
                );
                (subject, text, html)
            }

            EmailNotificationType::Message => {
                let actor = vars.actor_name.as_deref().unwrap_or("Someone");
                let subject = format!("New message from {} on {}", actor, config.instance_name);
                let text = format!(
                    "You have a new message from {}.\n\nLog in to read it: {}/messaging",
                    actor, config.instance_url
                );
                let html = self.wrap_html(
                    &format!(
                        "<p>You have a new message from <strong>{}</strong>.</p>\
                    <p><a href=\"{}/messaging\">Log in to read it</a></p>",
                        actor, config.instance_url
                    ),
                    config,
                );
                (subject, text, html)
            }

            EmailNotificationType::PasswordReset => {
                let subject = format!("Reset your password on {}", config.instance_name);
                let action_url = vars.action_url.as_deref().unwrap_or("");
                let text = format!(
                    "You requested a password reset for your account on {}.\n\n\
                    Click the following link to reset your password:\n{}\n\n\
                    If you didn't request this, you can safely ignore this email.",
                    config.instance_name, action_url
                );
                let html = self.wrap_html(&format!(
                    "<p>You requested a password reset for your account on {}.</p>\
                    <p><a href=\"{}\" style=\"display:inline-block;padding:12px 24px;background:#007bff;color:#fff;text-decoration:none;border-radius:4px;\">Reset Password</a></p>\
                    <p><small>If you didn't request this, you can safely ignore this email.</small></p>",
                    config.instance_name,
                    action_url
                ), config);
                (subject, text, html)
            }

            EmailNotificationType::EmailVerification => {
                let subject = format!("Verify your email on {}", config.instance_name);
                let action_url = vars.action_url.as_deref().unwrap_or("");
                let text = format!(
                    "Please verify your email address for your account on {}.\n\n\
                    Click the following link to verify:\n{}\n\n\
                    Or enter this code: {}",
                    config.instance_name,
                    action_url,
                    vars.action_code.as_deref().unwrap_or("")
                );
                let html = self.wrap_html(&format!(
                    "<p>Please verify your email address for your account on {}.</p>\
                    <p><a href=\"{}\" style=\"display:inline-block;padding:12px 24px;background:#28a745;color:#fff;text-decoration:none;border-radius:4px;\">Verify Email</a></p>\
                    <p>Or enter this code: <strong>{}</strong></p>",
                    config.instance_name,
                    action_url,
                    vars.action_code.as_deref().unwrap_or("")
                ), config);
                (subject, text, html)
            }

            EmailNotificationType::WeeklyDigest => {
                let subject = format!("Your weekly digest from {}", config.instance_name);
                let count = vars.item_count.unwrap_or(0);
                let text = format!(
                    "Here's what happened on {} this week:\n\n\
                    {} new notifications\n\n\
                    Log in to see more: {}",
                    config.instance_name, count, config.instance_url
                );
                let html = self.wrap_html(
                    &format!(
                        "<p>Here's what happened on {} this week:</p>\
                    <p><strong>{}</strong> new notifications</p>\
                    <p><a href=\"{}\">Log in to see more</a></p>",
                        config.instance_name, count, config.instance_url
                    ),
                    config,
                );
                (subject, text, html)
            }

            EmailNotificationType::MonthlyDigest => {
                let subject = format!("Your monthly digest from {}", config.instance_name);
                let count = vars.item_count.unwrap_or(0);
                let text = format!(
                    "Here's your monthly summary from {}:\n\n\
                    {} activities this month\n\n\
                    Log in to see more: {}",
                    config.instance_name, count, config.instance_url
                );
                let html = self.wrap_html(
                    &format!(
                        "<p>Here's your monthly summary from {}:</p>\
                    <p><strong>{}</strong> activities this month</p>\
                    <p><a href=\"{}\">Log in to see more</a></p>",
                        config.instance_name, count, config.instance_url
                    ),
                    config,
                );
                (subject, text, html)
            }

            EmailNotificationType::SecurityAlert => {
                let subject = format!("Security alert from {}", config.instance_name);
                let text = format!(
                    "We detected unusual activity on your {} account.\n\n\
                    If this wasn't you, please change your password immediately: {}/settings/security",
                    config.instance_name, config.instance_url
                );
                let html = self.wrap_html(&format!(
                    "<p style=\"color:#dc3545;\"><strong>Security Alert</strong></p>\
                    <p>We detected unusual activity on your {} account.</p>\
                    <p>If this wasn't you, please <a href=\"{}/settings/security\">change your password</a> immediately.</p>",
                    config.instance_name,
                    config.instance_url
                ), config);
                (subject, text, html)
            }

            EmailNotificationType::Welcome => {
                let user_name = vars.user_name.as_deref().unwrap_or("there");
                let subject = format!("Welcome to {}!", config.instance_name);
                let text = format!(
                    "Hi {}!\n\n\
                    Welcome to {}! We're glad to have you.\n\n\
                    Get started: {}\n\n\
                    If you have any questions, feel free to reach out.",
                    user_name, config.instance_name, config.instance_url
                );
                let html = self.wrap_html(&format!(
                    "<p>Hi {}!</p>\
                    <p>Welcome to <strong>{}</strong>! We're glad to have you.</p>\
                    <p><a href=\"{}\" style=\"display:inline-block;padding:12px 24px;background:#007bff;color:#fff;text-decoration:none;border-radius:4px;\">Get Started</a></p>\
                    <p>If you have any questions, feel free to reach out.</p>",
                    user_name,
                    config.instance_name,
                    config.instance_url
                ), config);
                (subject, text, html)
            }
        };

        Ok((subject, text, html))
    }

    /// Wrap HTML content in a basic email template.
    fn wrap_html(&self, content: &str, config: &EmailConfig) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px; }}
        a {{ color: #007bff; }}
        blockquote {{ margin: 10px 0; padding: 10px 20px; border-left: 4px solid #e9ecef; background: #f8f9fa; }}
    </style>
</head>
<body>
    {}
    <hr style="margin-top: 40px; border: none; border-top: 1px solid #e9ecef;">
    <p style="font-size: 12px; color: #6c757d;">
        This email was sent from <a href="{}">{}</a>.<br>
        You can manage your email preferences in your account settings.
    </p>
</body>
</html>"#,
            content, config.instance_url, config.instance_name
        )
    }

    // Provider-specific implementations

    async fn send_smtp(
        &self,
        smtp: &SmtpConfig,
        config: &EmailConfig,
        message: EmailMessage,
    ) -> AppResult<EmailDeliveryResult> {
        // Parse from address
        let from_mailbox: Mailbox = format!("{} <{}>", config.from_name, config.from_address)
            .parse()
            .map_err(|e| AppError::Validation(format!("Invalid from address: {e}")))?;

        // Parse to address
        let to_mailbox: Mailbox = message
            .to
            .parse()
            .map_err(|e| AppError::Validation(format!("Invalid to address: {e}")))?;

        // Build email message
        let mut email_builder = Message::builder()
            .from(from_mailbox)
            .to(to_mailbox)
            .subject(&message.subject);

        // Add reply-to if specified
        if let Some(reply_to) = message.reply_to.as_ref().or(config.reply_to.as_ref()) {
            let reply_to_mailbox: Mailbox = reply_to
                .parse()
                .map_err(|e| AppError::Validation(format!("Invalid reply-to address: {e}")))?;
            email_builder = email_builder.reply_to(reply_to_mailbox);
        }

        // Build message body
        let email = if let Some(html_body) = message.html_body {
            // Multipart email with both text and HTML
            email_builder
                .multipart(
                    MultiPart::alternative()
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_PLAIN)
                                .body(message.text_body),
                        )
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_HTML)
                                .body(html_body),
                        ),
                )
                .map_err(|e| AppError::Internal(format!("Failed to build email: {e}")))?
        } else {
            // Plain text only
            email_builder
                .body(message.text_body)
                .map_err(|e| AppError::Internal(format!("Failed to build email: {e}")))?
        };

        // Build SMTP transport
        if !smtp.use_tls {
            // For non-TLS, use the plain transport
            return self.send_smtp_plain(smtp, email).await;
        }

        let mut transport = AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp.host)
            .map_err(|e| AppError::ExternalService(format!("SMTP connection failed: {e}")))?
            .port(smtp.port);

        // Add credentials if provided
        if let (Some(username), Some(password)) = (&smtp.username, &smtp.password) {
            transport = transport.credentials(Credentials::new(username.clone(), password.clone()));
        }

        let mailer = transport.build();

        // Send email
        match mailer.send(email).await {
            Ok(response) => {
                tracing::info!(
                    to = %message.to,
                    subject = %message.subject,
                    "Email sent via SMTP"
                );
                Ok(EmailDeliveryResult {
                    success: true,
                    message_id: Some(response.message().next().map_or_else(
                        || format!("smtp-{}", uuid::Uuid::new_v4()),
                        std::string::ToString::to_string,
                    )),
                    error: None,
                })
            }
            Err(e) => {
                tracing::error!(error = %e, "SMTP send failed");
                Ok(EmailDeliveryResult {
                    success: false,
                    message_id: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Send email via plain (non-TLS) SMTP connection.
    async fn send_smtp_plain(
        &self,
        smtp: &SmtpConfig,
        email: Message,
    ) -> AppResult<EmailDeliveryResult> {
        let mut transport =
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&smtp.host).port(smtp.port);

        // Add credentials if provided
        if let (Some(username), Some(password)) = (&smtp.username, &smtp.password) {
            transport = transport.credentials(Credentials::new(username.clone(), password.clone()));
        }

        let mailer = transport.build();

        match mailer.send(email).await {
            Ok(response) => Ok(EmailDeliveryResult {
                success: true,
                message_id: Some(response.message().next().map_or_else(
                    || format!("smtp-{}", uuid::Uuid::new_v4()),
                    std::string::ToString::to_string,
                )),
                error: None,
            }),
            Err(e) => {
                tracing::error!(error = %e, "SMTP send failed");
                Ok(EmailDeliveryResult {
                    success: false,
                    message_id: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    async fn send_ses(
        &self,
        ses: &SesConfig,
        config: &EmailConfig,
        message: EmailMessage,
    ) -> AppResult<EmailDeliveryResult> {
        // Build raw email using lettre for proper MIME formatting
        let from_mailbox: Mailbox = format!("{} <{}>", config.from_name, config.from_address)
            .parse()
            .map_err(|e| AppError::Validation(format!("Invalid from address: {e}")))?;

        let to_mailbox: Mailbox = message
            .to
            .parse()
            .map_err(|e| AppError::Validation(format!("Invalid to address: {e}")))?;

        let mut email_builder = Message::builder()
            .from(from_mailbox)
            .to(to_mailbox)
            .subject(&message.subject);

        if let Some(reply_to) = message.reply_to.as_ref().or(config.reply_to.as_ref()) {
            let reply_to_mailbox: Mailbox = reply_to
                .parse()
                .map_err(|e| AppError::Validation(format!("Invalid reply-to address: {e}")))?;
            email_builder = email_builder.reply_to(reply_to_mailbox);
        }

        let email = if let Some(html_body) = message.html_body {
            email_builder
                .multipart(
                    MultiPart::alternative()
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_PLAIN)
                                .body(message.text_body.clone()),
                        )
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_HTML)
                                .body(html_body),
                        ),
                )
                .map_err(|e| AppError::Internal(format!("Failed to build email: {e}")))?
        } else {
            email_builder
                .body(message.text_body.clone())
                .map_err(|e| AppError::Internal(format!("Failed to build email: {e}")))?
        };

        // Get raw email content
        let raw_email = email.formatted();
        let raw_email_base64 =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &raw_email);

        // Build SES v1 API request
        let endpoint = format!("https://email.{}.amazonaws.com", ses.region);
        let timestamp = chrono::Utc::now();
        let date_stamp = timestamp.format("%Y%m%d").to_string();
        let amz_date = timestamp.format("%Y%m%dT%H%M%SZ").to_string();

        // Create AWS signature v4
        let (authorization, signed_headers) =
            self.create_ses_signature(ses, &endpoint, &amz_date, &date_stamp, &raw_email_base64)?;

        // Build request body
        let body = format!(
            "Action=SendRawEmail&RawMessage.Data={}",
            urlencoding::encode(&raw_email_base64)
        );

        // Send request
        let response = self
            .http_client
            .post(&endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("X-Amz-Date", &amz_date)
            .header("Host", format!("email.{}.amazonaws.com", ses.region))
            .header("Authorization", authorization)
            .header("X-Amz-Content-Sha256", signed_headers)
            .body(body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("SES request failed: {e}")))?;

        if response.status().is_success() {
            // Parse response XML to get message ID
            let response_text = response.text().await.unwrap_or_default();
            let message_id = self.extract_ses_message_id(&response_text);

            tracing::info!(
                to = %message.to,
                subject = %message.subject,
                "Email sent via SES"
            );

            Ok(EmailDeliveryResult {
                success: true,
                message_id,
                error: None,
            })
        } else {
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!(error = %error_text, "SES send failed");
            Ok(EmailDeliveryResult {
                success: false,
                message_id: None,
                error: Some(error_text),
            })
        }
    }

    /// Create AWS Signature Version 4 for SES.
    fn create_ses_signature(
        &self,
        ses: &SesConfig,
        _endpoint: &str,
        amz_date: &str,
        date_stamp: &str,
        payload: &str,
    ) -> AppResult<(String, String)> {
        use hmac::{Hmac, Mac};
        use sha2::{Digest, Sha256};

        // Step 1: Create canonical request
        let host = format!("email.{}.amazonaws.com", ses.region);
        let payload_hash = hex::encode(Sha256::digest(payload.as_bytes()));

        let canonical_request = format!(
            "POST\n/\n\ncontent-type:application/x-www-form-urlencoded\nhost:{host}\nx-amz-date:{amz_date}\n\ncontent-type;host;x-amz-date\n{payload_hash}"
        );

        // Step 2: Create string to sign
        let credential_scope = format!("{}/{}/ses/aws4_request", date_stamp, ses.region);
        let canonical_request_hash = hex::encode(Sha256::digest(canonical_request.as_bytes()));
        let string_to_sign =
            format!("AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{canonical_request_hash}");

        // Step 3: Calculate signature
        type HmacSha256 = Hmac<Sha256>;

        let k_secret = format!("AWS4{}", ses.secret_access_key);
        let mut mac = HmacSha256::new_from_slice(k_secret.as_bytes())
            .map_err(|e| AppError::Internal(format!("HMAC error: {e}")))?;
        mac.update(date_stamp.as_bytes());
        let k_date = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&k_date)
            .map_err(|e| AppError::Internal(format!("HMAC error: {e}")))?;
        mac.update(ses.region.as_bytes());
        let k_region = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&k_region)
            .map_err(|e| AppError::Internal(format!("HMAC error: {e}")))?;
        mac.update(b"ses");
        let k_service = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&k_service)
            .map_err(|e| AppError::Internal(format!("HMAC error: {e}")))?;
        mac.update(b"aws4_request");
        let k_signing = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&k_signing)
            .map_err(|e| AppError::Internal(format!("HMAC error: {e}")))?;
        mac.update(string_to_sign.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        // Step 4: Build authorization header
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders=content-type;host;x-amz-date, Signature={}",
            ses.access_key_id, credential_scope, signature
        );

        Ok((authorization, payload_hash))
    }

    /// Extract message ID from SES XML response.
    fn extract_ses_message_id(&self, response: &str) -> Option<String> {
        // Simple XML parsing for <MessageId>...</MessageId>
        let start = response.find("<MessageId>")? + 11;
        let end = response[start..].find("</MessageId>")? + start;
        Some(response[start..end].to_string())
    }

    async fn send_sendgrid(
        &self,
        sg: &SendGridConfig,
        config: &EmailConfig,
        message: EmailMessage,
    ) -> AppResult<EmailDeliveryResult> {
        let body = serde_json::json!({
            "personalizations": [{
                "to": [{"email": message.to}]
            }],
            "from": {
                "email": config.from_address,
                "name": config.from_name
            },
            "subject": message.subject,
            "content": [
                {"type": "text/plain", "value": message.text_body},
                {"type": "text/html", "value": message.html_body.unwrap_or_default()}
            ]
        });

        let response = self
            .http_client
            .post("https://api.sendgrid.com/v3/mail/send")
            .header("Authorization", format!("Bearer {}", sg.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("SendGrid request failed: {e}")))?;

        if response.status().is_success() {
            let message_id = response
                .headers()
                .get("X-Message-Id")
                .and_then(|v| v.to_str().ok())
                .map(String::from);
            Ok(EmailDeliveryResult {
                success: true,
                message_id,
                error: None,
            })
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Ok(EmailDeliveryResult {
                success: false,
                message_id: None,
                error: Some(error_text),
            })
        }
    }

    async fn send_mailgun(
        &self,
        mg: &MailgunConfig,
        config: &EmailConfig,
        message: EmailMessage,
    ) -> AppResult<EmailDeliveryResult> {
        let base_url = if mg.eu_region {
            "https://api.eu.mailgun.net"
        } else {
            "https://api.mailgun.net"
        };

        let mut form_params = vec![
            (
                "from",
                format!("{} <{}>", config.from_name, config.from_address),
            ),
            ("to", message.to),
            ("subject", message.subject),
            ("text", message.text_body),
        ];

        if let Some(html) = message.html_body {
            form_params.push(("html", html));
        }

        let response = self
            .http_client
            .post(format!("{}/v3/{}/messages", base_url, mg.domain))
            .basic_auth("api", Some(&mg.api_key))
            .form(&form_params)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Mailgun request failed: {e}")))?;

        if response.status().is_success() {
            #[derive(Deserialize)]
            struct MailgunResponse {
                id: Option<String>,
            }
            let result: MailgunResponse = response
                .json()
                .await
                .unwrap_or(MailgunResponse { id: None });
            Ok(EmailDeliveryResult {
                success: true,
                message_id: result.id,
                error: None,
            })
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Ok(EmailDeliveryResult {
                success: false,
                message_id: None,
                error: Some(error_text),
            })
        }
    }
}

/// Response for email service status.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailStatusResponse {
    /// Whether email service is available
    pub available: bool,
    /// Provider name
    pub provider: Option<String>,
}
