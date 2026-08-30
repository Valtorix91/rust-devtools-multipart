use devtools_multipart_service::{MultipartClient, UploadPlan, choose_part_count};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes: u64 = std::env::args().nth(1).unwrap_or_else(|| "10485760".into()).parse()?;
    let plan = UploadPlan { bucket: "devtools-media".into(), object_key: "builds/latest/archive.bin".into(), parts: choose_part_count(bytes, 5_242_880) };
    let client = MultipartClient::from_env()?;
    client.prepare(&plan).await?;
    println!("prepared {} parts for {}", plan.parts, plan.object_key);
    Ok(())
}

