mod cli;
mod s3;

use anyhow::Result;
use bytesize::ByteSize;
use clap::Parser;
use cli_table::{
    format::{Border, Justify, Separator},
    Cell, Table,
};
use futures_util::{stream::FuturesUnordered, TryStreamExt};

use crate::cli::{Args, Command};

async fn list(s3_client: &aws_sdk_s3::Client, bucket: &str) -> Result<()> {
    let metadatas = s3::list_metadatas(s3_client, bucket).await?;
    let table = metadatas
        .into_iter()
        .map(|(id, metadata)| {
            vec![
                id.cell(),
                metadata.creator_email.cell(),
                ByteSize(metadata.size as u64)
                    .to_string()
                    .cell()
                    .justify(Justify::Right),
                ByteSize(metadata.block_size as u64)
                    .to_string()
                    .cell()
                    .justify(Justify::Right),
            ]
        })
        .collect::<Vec<_>>()
        .table()
        .title(vec![
            "ID".cell(),
            "CREATOR EMAIL".cell(),
            "SIZE".cell(),
            "BLOCK SIZE".cell(),
        ])
        .separator(
            Separator::builder()
                .column(None)
                .row(None)
                .title(None)
                .build(),
        )
        .border(Border::builder().build());
    cli_table::print_stdout(table)?;
    Ok(())
}

async fn delete(s3_client: &aws_sdk_s3::Client, bucket: &str, ids: &[String]) -> Result<()> {
    ids.iter()
        .map(|id| async move {
            s3::delete_metadata(s3_client, bucket, id).await?;
            s3::delete_file(s3_client, bucket, id).await?;
            println!("{id} deleted");
            Ok(())
        })
        .collect::<FuturesUnordered<_>>()
        .try_collect::<()>()
        .await
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let aws_config = aws_config::load_from_env().await;
    let s3_client = aws_sdk_s3::Client::new(&aws_config);

    match args.command {
        Command::List => list(&s3_client, &args.bucket).await?,
        Command::Delete { ids } => delete(&s3_client, &args.bucket, &ids).await?,
    }

    Ok(())
}
