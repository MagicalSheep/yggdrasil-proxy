use once_cell::race::OnceBox;
use sea_orm::*;
use std::time::Duration;
use crate::entity::prelude::Profiles;
use crate::entity::profiles;

static DB: OnceBox<DatabaseConnection> = OnceBox::new();

/// Initialize database connection, and try creating tables.
/// It will be ignored if creating tables failed.
pub async fn init(data_source: &str) -> Result<(), DbErr> {
    let mut opt = ConnectOptions::new(data_source.to_string());
    opt.max_connections(100)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(false)
        .sqlx_logging_level(log::LevelFilter::Info);
    match Database::connect(opt).await {
        Ok(db) => {
            DB.set(Box::new(db)).unwrap();
        }
        Err(err) => { return Err(err); }
    }
    let _ = create_table().await;
    Ok(())
}

/// Create tables and its index.
async fn create_table() -> Result<(), DbErr> {
    let db = DB.get().unwrap();
    let builder = db.get_database_backend();
    let schema = Schema::new(builder);
    if let Err(err) = db.execute(builder.build(&schema.create_table_from_entity(Profiles))).await {
        return Err(err);
    }
    let src_index = sea_query::Index::create()
        .name("src_index")
        .table(Profiles)
        .col(profiles::Column::BackendId)
        .col(profiles::Column::SrcName)
        .unique()
        .to_owned();
    let src_id_index = sea_query::Index::create()
        .name("src_id_index")
        .table(Profiles)
        .col(profiles::Column::BackendId)
        .col(profiles::Column::SrcUuid)
        .unique()
        .to_owned();
    if let Err(err) = db.execute(builder.build(&src_index)).await {
        return Err(err);
    };
    if let Err(err) = db.execute(builder.build(&src_id_index)).await {
        return Err(err);
    };
    Ok(())
}

pub async fn find_by_backend_and_uuid(backend_id: &str, src_uuid: &str) -> Result<Option<profiles::Model>, DbErr> {
    let db = DB.get().unwrap();
    Profiles::find()
        .filter(profiles::Column::BackendId.eq(backend_id))
        .filter(profiles::Column::SrcUuid.eq(src_uuid))
        .one(db)
        .await
}

pub async fn find_by_name(name: &str) -> Result<Option<profiles::Model>, DbErr> {
    let db = DB.get().unwrap();
    Profiles::find()
        .filter(profiles::Column::Name.eq(name))
        .one(db)
        .await
}

pub async fn find_by_uuid(uuid: &str) -> Result<Option<profiles::Model>, DbErr> {
    let db = DB.get().unwrap();
    Profiles::find()
        .filter(profiles::Column::Uuid.eq(uuid))
        .one(db)
        .await
}

pub async fn save_profile(profile: profiles::ActiveModel) -> Result<profiles::ActiveModel, DbErr> {
    let db = DB.get().unwrap();
    let res: profiles::ActiveModel = profile.save(db).await?;
    Ok(res)
}

// pub async fn del_profile(profile: profiles::ActiveModel) -> Result<DeleteResult, DbErr> {
//     let db = DB.get().unwrap();
//     Ok(profile.delete(db).await?)
// }
