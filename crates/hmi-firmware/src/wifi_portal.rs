use anyhow::{anyhow, Result};
use embedded_io::Write;
use embedded_svc::http::Method;
use esp_idf_svc::http::server::{Configuration, EspHttpServer};
use hmi_core::wifi_setup::{parse_form, Credentials};
use std::sync::mpsc::{sync_channel, Receiver};

const FORM: &[u8] = br#"<!doctype html><html lang="en"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Living Display Wi-Fi</title><style>body{font:19px system-ui;background:#f8f6ef;color:#1b2926;max-width:32em;margin:3em auto;padding:0 1.2em}input,button{font:inherit;width:100%;box-sizing:border-box;padding:.7em;margin:.3em 0 1.3em;border:1px solid #35554d;border-radius:6px}button{background:#143f36;color:white}small{line-height:1.5}</style><h1>Connect your Living Display</h1><p>Enter a 2.4 GHz Wi-Fi network. The display will save it only after it connects.</p><form method="post" action="/save"><label>Network name<input name="ssid" maxlength="32" required autocomplete="off"></label><label>Password<input type="password" name="password" maxlength="63" autocomplete="off"></label><button>Connect</button></form><small>Leave the display powered on while it connects. This setup network closes after three minutes or when setup finishes.</small></html>"#;

pub struct Portal {
    _server: EspHttpServer<'static>,
    rx: Receiver<Credentials>,
}

impl Portal {
    pub fn start() -> Result<Self> {
        let (tx, rx) = sync_channel(1);
        let mut server = EspHttpServer::new(&Configuration::default())?;
        server.fn_handler("/", Method::Get, |request| -> Result<()> {
            let mut response = request.into_response(
                200,
                Some("OK"),
                &[
                    ("Content-Type", "text/html; charset=utf-8"),
                    ("Cache-Control", "no-store"),
                ],
            )?;
            response.write_all(FORM)?;
            Ok(())
        })?;
        server.fn_handler("/save", Method::Post, move |mut request| -> Result<()> {
            let length = request
                .header("Content-Length")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            if length == 0 || length > 256 {
                request
                    .into_status_response(413)?
                    .write_all(b"Invalid form size")?;
                return Ok(());
            }
            let mut body = [0u8; 256];
            let mut used = 0;
            while used < length {
                let read = request.read(&mut body[used..length])?;
                if read == 0 {
                    return Err(anyhow!("incomplete setup form"));
                }
                used += read;
            }
            if parse_form(&body[..length]).is_some_and(|credentials| tx.try_send(credentials).is_ok())
            {
                request
                    .into_response(
                        200,
                        Some("OK"),
                        &[
                            ("Content-Type", "text/html; charset=utf-8"),
                            ("Cache-Control", "no-store"),
                        ],
                    )?
                    .write_all(b"<html><meta name='viewport' content='width=device-width,initial-scale=1'><h1>Connecting...</h1><p>Watch your display for the result.</p></html>")?;
            } else {
                request
                    .into_status_response(400)?
                    .write_all(b"Invalid network name or password")?;
            }
            Ok(())
        })?;
        Ok(Self {
            _server: server,
            rx,
        })
    }

    pub fn receive(&self) -> Option<Credentials> {
        self.rx.try_recv().ok()
    }
}
