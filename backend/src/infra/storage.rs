use aws_credential_types::Credentials;
use aws_sdk_s3::{
    config::{BehaviorVersion, Region},
    primitives::ByteStream,
    Client,
};
use tracing::instrument;
use uuid::Uuid;

use crate::config::Config;

#[derive(Clone)]
pub struct ObjectStore {
    client: Client,
    bucket: String,
}

impl ObjectStore {
    pub async fn new(cfg: &Config) -> anyhow::Result<Self> {
        let creds = Credentials::new(
            &cfg.garage_access_key,
            &cfg.garage_secret_key,
            None,
            None,
            "static",
        );
        let conf = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(cfg.garage_region.clone()))
            .endpoint_url(&cfg.garage_endpoint)
            .credentials_provider(creds)
            .force_path_style(true)
            .build();
        Ok(Self {
            client: Client::from_conf(conf),
            bucket: cfg.garage_bucket.clone(),
        })
    }

    #[instrument(skip(self, bytes), fields(bucket = %self.bucket, key))]
    pub async fn put_object(
        &self,
        key: &str,
        bytes: Vec<u8>,
        content_type: &str,
    ) -> anyhow::Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(bytes))
            .content_type(content_type)
            .send()
            .await?;
        Ok(())
    }

    pub fn new_equipment_image_key(equipment_id: Uuid, filename: &str) -> String {
        let safe = filename.replace(['/', '\\'], "_");
        format!("equipment/{equipment_id}/{safe}")
    }
}
