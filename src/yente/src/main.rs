use tokio_postgres::NoTls;

#[tokio::main]
async fn main() {
    let (client, connection) = tokio_postgres::connect("host=localhost user=robot", NoTls)
        .await
        .unwrap();

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e)
        }
    });

    let rows = client.query("SELECT * FROM users;", &[]).await.unwrap();

    let name: &str = rows[0].get(1);
    println!("{}", name);
}
