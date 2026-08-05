// Módulo de envío de correos electrónicos vía SMTP.
// Configuración a través de variables de entorno:
//   SMTP_HOST, SMTP_PORT (587 por defecto), SMTP_USERNAME, SMTP_PASSWORD, SMTP_FROM (opcional)

use anyhow::{Context, Result};
use lettre::{
    message::{header::ContentType, Attachment, Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

/// Envía un correo con un archivo PDF adjunto.
pub async fn enviar_correo_con_pdf(
    destinatario: &str,
    asunto: &str,
    cuerpo: &str,
    pdf_bytes: Vec<u8>,
    nombre_pdf: &str,
) -> Result<()> {
    let host = std::env::var("SMTP_HOST").context("Variable SMTP_HOST no configurada")?;
    let port: u16 = std::env::var("SMTP_PORT")
        .unwrap_or_else(|_| "587".to_string())
        .parse()
        .context("SMTP_PORT inválido")?;
    let username =
        std::env::var("SMTP_USERNAME").context("Variable SMTP_USERNAME no configurada")?;
    let password =
        std::env::var("SMTP_PASSWORD").context("Variable SMTP_PASSWORD no configurada")?;
    let from = std::env::var("SMTP_FROM").unwrap_or_else(|_| username.clone());

    let from_mailbox: Mailbox = from
        .parse()
        .context("La dirección remitente (SMTP_FROM) es inválida")?;
    let to_mailbox: Mailbox = destinatario
        .parse()
        .context("La dirección de correo del destinatario es inválida")?;

    // Cuerpo en texto plano
    let body = SinglePart::builder()
        .header(ContentType::TEXT_PLAIN)
        .body(cuerpo.to_string());

    // PDF adjunto
    let attachment = Attachment::new(nombre_pdf.to_string()).body(
        pdf_bytes,
        ContentType::parse("application/pdf").context("Tipo de contenido PDF inválido")?,
    );

    let email = Message::builder()
        .from(from_mailbox)
        .to(to_mailbox)
        .subject(asunto)
        .multipart(MultiPart::mixed().singlepart(body).singlepart(attachment))
        .context("Error al construir el mensaje de correo")?;

    // Puerto 465 = TLS implícito, resto = STARTTLS
    let builder = if port == 465 {
        AsyncSmtpTransport::<Tokio1Executor>::relay(&host)?
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)?
    };

    let mailer = builder
        .port(port)
        .credentials(Credentials::new(username, password))
        .build();

    mailer
        .send(email)
        .await
        .context("Error al enviar el correo")?;

    Ok(())
}
