use actix_web::{web, App, HttpResponse, HttpServer};
use dotenvy::dotenv;
use near_api::signer::Signer;
use near_api::{Account, Contract, NetworkConfig};
use near_primitives::types::AccountId;
use near_primitives::views::ExecutionStatusView;
use near_token::NearToken;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;

#[derive(Deserialize)]
struct GenerateRequest {
    background: String,
    body_color: String,
    eyes: String,
    special_traits: String,
    account_id: AccountId,
}

#[derive(Serialize)]
struct GenerateResponse {
    image_url: String,
    token_id: String,
}

#[derive(Deserialize)]
struct FalResponse {
    images: Vec<FalImage>,
}

#[derive(Deserialize)]
struct FalImage {
    url: String,
}

async fn generate(req: web::Json<GenerateRequest>, client: web::Data<Client>) -> HttpResponse {
    let prompt = format!(
        "Cartoonish anime {} dragon with {} eyes in {}, autumn, {}",
        req.body_color, req.eyes, req.background, req.special_traits
    );

    println!("Generating prompt: {}", prompt);

    let fal_key = env::var("FAL_KEY").expect("FAL_KEY must be set");

    let response = client
        .post("https://fal.run/fal-ai/flux-general")
        .header("Authorization", format!("Key {}", fal_key))
        .json(&serde_json::json!({
            "prompt": prompt,
            "image_size": "square_hd",
            "num_images": 1,
            "controlnets": [
                {
                    "path": "https://huggingface.co/promeai/FLUX.1-controlnet-lineart-promeai/resolve/main/diffusion_pytorch_model.safetensors?download=true",
                    "config_url": "https://huggingface.co/promeai/FLUX.1-controlnet-lineart-promeai/resolve/main/config.json?download=true",
                    "end_percentage": 0.9,
                    "mask_image_url": "https://i.imgur.com/oj9ha85.png",
                    "start_percentage": 0,
                    "control_image_url": "https://i.imgur.com/S8wOb4S.png",
                    "conditioning_scale": 0.8
                }
            ],
            "ip_adapters": [],
            "reference_end": 1,
            "guidance_scale": 3.5,
            "real_cfg_scale": 3.5,
            "controlnet_unions": [],
            "reference_strength": 0.65,
            "num_inference_steps": 28,
            "enable_safety_checker": false,
            "sync_mode": true,
        }))
        .send()
        .await;

    match response {
        Ok(res) => match res.json::<FalResponse>().await {
            Ok(fal_response) => {
                if let Some(first_image) = fal_response.images.first() {
                    let relayer = Account(
                        std::env::var("ACCOUNT")
                            .expect("ACCOUNT must be set")
                            .parse()
                            .expect("Failed to parse ACCOUNT"),
                    );
                    let signer = Signer::new(Signer::secret_key(
                        std::env::var("PRIVATE_KEY")
                            .expect("PRIVATE_KEY must be set")
                            .parse()
                            .expect("Failed to parse PRIVATE_KEY"),
                    ))
                    .expect("Failed to create signer");
                    let contract = Contract("nearvember-nft.near".parse().unwrap());
                    let tx = contract
                        .call_function(
                            "mint_for",
                            serde_json::json!({
                                "media_url": first_image.url,
                                "receiver_id": req.account_id,
                            }),
                        )
                        .expect("Failed to create mint_for call")
                        .transaction()
                        .deposit(NearToken::from_millinear(10)) // 0.01 NEAR
                        .with_signer(relayer.0, signer)
                        .send_to(&NetworkConfig {
                            rpc_url: "https://rpc.shitzuapes.xyz".parse().unwrap(),
                            ..NetworkConfig::mainnet()
                        })
                        .await
                        .expect("Failed to send mint transaction");
                    if let ExecutionStatusView::SuccessReceiptId(receipt_id) =
                        &tx.transaction_outcome.outcome.status
                    {
                        if let ExecutionStatusView::SuccessValue(value) = &tx
                            .receipts_outcome
                            .iter()
                            .find(|r| r.id == *receipt_id)
                            .expect("Failed to find receipt")
                            .outcome
                            .status
                        {
                            HttpResponse::Ok().json(GenerateResponse {
                                image_url: first_image.url.clone(),
                                token_id: serde_json::from_slice::<String>(value)
                                    .expect("Failed to parse token ID"),
                            })
                        } else {
                            HttpResponse::InternalServerError().json(serde_json::json!({
                                "error": "Failed to get token ID receipt"
                            }))
                        }
                    } else {
                        HttpResponse::InternalServerError().json(serde_json::json!({
                            "error": format!("Failed to mint token: {:?}", tx.transaction_outcome.outcome)
                        }))
                    }
                } else {
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": "No images generated"
                    }))
                }
            }
            Err(e) => {
                println!("Error parsing response: {}", e);
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Failed to parse response"
                }))
            }
        },
        Err(e) => {
            println!("Error making request: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to generate image"
            }))
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    // Create a client with a timeout
    let client = Client::builder()
        .timeout(Duration::from_secs(180_000))
        .build()
        .expect("Failed to create HTTP client");

    println!("Starting server at http://0.0.0.0:80");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(client.clone()))
            .service(web::resource("/generate").route(web::post().to(generate)))
    })
    .bind(("0.0.0.0", 80))?
    .run()
    .await
}
