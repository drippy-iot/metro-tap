use embedded_svc::{
    http::client::asynch::{Client, TrivialUnblockingConnection},
    utils::io::asynch::try_read_full,
};
use esp_idf_svc::{errors::EspIOError, http::client::EspHttpConnection};
use esp_idf_sys::EspError;

pub type HttpClient = Client<TrivialUnblockingConnection<EspHttpConnection>>;

pub async fn report_to_server(http: &mut HttpClient, buf: &mut [u8]) -> Result<usize, EspError> {
    // TODO: remove this hard-coded value
    let mut res = http
        .get("http://192.168.137.1/index.html")
        .await
        .map_err(|EspIOError(err)| err)?
        .submit()
        .await
        .map_err(|EspIOError(err)| err)?;
    let (_, body) = res.split();
    log::info!("connection opened to the resource");
    try_read_full(body, buf).await.map_err(|(EspIOError(err), _)| err)
}
