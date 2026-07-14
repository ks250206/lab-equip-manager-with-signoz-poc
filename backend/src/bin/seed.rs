use chrono::{Duration, Utc};
use equipment_reservation::{
    auth::hash_password,
    config::Config,
    infra::Db,
};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let config = Config::from_env()?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    let db = Db::new(pool);

    let admin_hash = hash_password("admin-password-1", &config.password_pepper)?;
    let user_hash = hash_password("user-password-12", &config.password_pepper)?;

    let admin = match db.find_user_by_email("admin@example.com").await? {
        Some(u) => u,
        None => db
            .create_user("admin@example.com", &admin_hash, "admin")
            .await?,
    };
    let user = match db.find_user_by_email("user@example.com").await? {
        Some(u) => u,
        None => {
            db.create_user("user@example.com", &user_hash, "user")
                .await?
        }
    };

    let equipment = if db.list_equipment().await?.is_empty() {
        db.create_equipment(
            "CNC Mill A1",
            "3-axis milling machine for metal prototypes",
            "Lab 2 / Bay 3",
            admin.id,
            None,
        )
        .await?
    } else {
        db.list_equipment().await?.into_iter().next().unwrap()
    };

    let starts = Utc::now() + Duration::hours(24);
    let ends = starts + Duration::hours(2);
    if !db
        .has_overlapping_reservation(equipment.id, starts, ends)
        .await?
    {
        let _ = db
            .create_reservation(equipment.id, user.id, starts, ends)
            .await?;
    }

    println!("Seed complete:");
    println!("  admin@example.com / admin-password-1");
    println!("  user@example.com  / user-password-12");
    println!("  equipment: {} ({})", equipment.name, equipment.id);
    Ok(())
}
