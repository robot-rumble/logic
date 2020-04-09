use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let pool = PgPool::builder()
        .max_size(5)
        .build(&dotenv::var("DATABASE_URL")?)
        .await?;

    let users = sqlx::query!("SELECT * FROM users;")
        .fetch_all(&pool)
        .await?;

    println!("{:?}", users);

    Ok(())
}
