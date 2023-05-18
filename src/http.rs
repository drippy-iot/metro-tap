use embedded_svc::{
    http::client::asynch::{Client, TrivialUnblockingConnection},
    io::asynch::Read as _,
};
use esp_idf_svc::http::client::EspHttpConnection;
use esp_idf_sys::EspError;

pub type HttpClient = Client<TrivialUnblockingConnection<EspHttpConnection>>;

pub async fn report_to_server(http: &mut HttpClient) -> Result<Vec<u8>, EspError> {
    // TODO: remove this hard-coded value
    let mut res = http
        .get("http://127.0.0.1/index.html")
        .await
        .map_err(|err| err.0)?
        .submit()
        .await
        .map_err(|err| err.0)?;
    let (_, body) = res.split();

    let mut buf = Vec::with_capacity(512);
    buf.resize(512, 0);

    log::info!("starting the response");
    let mut cursor = 0;
    loop {
        let size = body.read(&mut buf[cursor..]).await.map_err(|err| dbg!(err.0))?;
        log::info!("read {size} bytes");
        cursor += size;
        if size == 0 {
            break;
        }
    }

    log::info!("finishing the response");
    buf.truncate(cursor);
    Ok(buf)
}
