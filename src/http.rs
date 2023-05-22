use embedded_svc::{
    http::client::asynch::{Client, TrivialUnblockingConnection},
    io::asynch::Write as _,
    utils::io::asynch::try_read_full,
};
use esp_idf_svc::{errors::EspIOError, http::client::EspHttpConnection};
use model::report::Flow;

pub type HttpClient = Client<TrivialUnblockingConnection<EspHttpConnection>>;

struct Post {
    count: usize,
    status: u16,
}

async fn send_post(http: &mut HttpClient, url: &str, data: &[u8], out: &mut [u8]) -> Result<Post, EspIOError> {
    let mut req = http.post(url, &[]).await?;
    req.write_all(data).await?;
    req.flush().await?;

    let mut res = req.submit().await?;
    let status = res.status();
    let (_, body) = res.split();

    let count = try_read_full(body, out).await.map_err(|(err, _)| err)?;
    Ok(Post { count, status })
}

pub async fn register_to_server(http: &mut HttpClient, mac: &[u8]) -> Result<bool, EspIOError> {
    const ENDPOINT: &str = concat!(env!("BASE_URL"), "/report/register");
    let mut buf = [];
    let Post { count, status } = send_post(http, ENDPOINT, mac, &mut buf).await?;
    assert_eq!(count, 0);
    Ok(match status {
        201 => true,
        503 => false,
        _ => core::unreachable!(),
    })
}

pub async fn report_flow(http: &mut HttpClient, flow: &Flow) -> Result<bool, EspIOError> {
    const ENDPOINT: &str = concat!(env!("BASE_URL"), "/report/flow");
    let bytes = model::encode(flow).unwrap();
    let mut buf = [];
    let Post { count, status } = send_post(http, ENDPOINT, &bytes, &mut buf).await?;
    assert_eq!(count, 0);
    Ok(match status {
        201 => true,
        503 => false,
        _ => core::unreachable!(),
    })
}

pub async fn report_leak(http: &mut HttpClient, mac: &[u8]) -> Result<bool, EspIOError> {
    const ENDPOINT: &str = concat!(env!("BASE_URL"), "/report/leak");
    let mut buf = [];
    let Post { count, status } = send_post(http, ENDPOINT, mac, &mut buf).await?;
    assert_eq!(count, 0);
    Ok(match status {
        201 => true,
        503 => false,
        _ => core::unreachable!(),
    })
}
