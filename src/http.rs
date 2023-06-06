use embedded_svc::{
    http::client::asynch::{Client, TrivialUnblockingConnection},
    io::asynch::Write as _,
    utils::io::asynch::try_read_full,
};
use esp_idf_svc::{errors::EspIOError, http::client::EspHttpConnection};
use model::report::Ping;

pub type HttpClient = Client<TrivialUnblockingConnection<EspHttpConnection>>;

/// Command from the server piggy-backed on the ping response.
pub enum Command {
    /// Do nothing.
    None,
    /// Open the valve.
    Open,
    /// Close the valve.
    Close,
}

struct Post {
    count: usize,
    status: u16,
}

async fn send_post(http: &mut HttpClient, url: &str, data: &[u8], out: &mut [u8]) -> Result<Post, EspIOError> {
    let len = data.len().to_string();
    let headers = [("Content-Length", len.as_str())];
    let mut req = http.post(url, &headers).await?;
    req.write_all(data).await?;
    req.flush().await?;

    let mut res = req.submit().await?;
    let status = res.status();
    let (_, body) = res.split();

    let count = try_read_full(body, out).await.map_err(|(err, _)| err)?;
    Ok(Post { count, status })
}

pub async fn register_to_server(http: &mut HttpClient, mac: &[u8]) -> Result<Command, EspIOError> {
    const ENDPOINT: &str = concat!(env!("BASE_URL"), "/report/register");
    let mut buf = [];
    let Post { count, status } = send_post(http, ENDPOINT, mac, &mut buf).await?;
    assert_eq!(count, 0);
    Ok(match status {
        201 => Command::None,
        204 => Command::Close,
        205 => Command::Open,
        _ => core::unreachable!(),
    })
}

pub async fn ping(http: &mut HttpClient, ping: &Ping) -> Result<Command, EspIOError> {
    const ENDPOINT: &str = concat!(env!("BASE_URL"), "/report/ping");
    let bytes = model::encode(ping).unwrap();
    let mut buf = [];
    let Post { count, status } = send_post(http, ENDPOINT, &bytes, &mut buf).await?;
    assert_eq!(count, 0);
    Ok(match status {
        201 => Command::None,
        204 => Command::Close,
        205 => Command::Open,
        _ => core::unreachable!(),
    })
}

pub async fn bypass(http: &mut HttpClient) -> Result<(), EspIOError> {
    const ENDPOINT: &str = concat!(env!("BASE_URL"), "/report/bypass");
    let mut buf = [];
    let Post { count, status } = send_post(http, ENDPOINT, &[], &mut buf).await?;
    assert_eq!(count, 0);
    assert_eq!(status, 201);
    Ok(())
}
