mod shortener;
mod store;
mod token;

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    color_eyre::install().expect("Failed to install color_eyre");
    Ok(shortener::create_router().into())
}
