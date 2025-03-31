use bevy::tasks::futures_lite::StreamExt;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_util::codec::{BytesCodec, FramedRead};
use uuid::Uuid;

use reqwest::{multipart, Client};

// Change the server address if needed.
pub const SERVER_ADDRESS: &str = "127.0.0.1:8188";

pub async fn connect_comfy(
) -> Result<(Client, String, WebSocketStream<MaybeTlsStream<TcpStream>>), Box<dyn Error>> {
    let client = Client::new();
    let client_id = Uuid::new_v4().to_string();
    let ws_url = format!("ws://{}/ws?clientId={}", SERVER_ADDRESS, client_id);
    let (ws, _response) = connect_async(ws_url).await?;
    Ok((client, client_id, ws))
}

/// Sends the prompt to the server and returns the JSON response.
pub async fn queue_prompt(
    client: &Client,
    prompt: &Value,
    client_id: &str,
) -> Result<Value, Box<dyn Error>> {
    let url = format!("http://{}/prompt", SERVER_ADDRESS);
    let payload = serde_json::json!({
        "prompt": prompt,
        "client_id": client_id,
    });
    let resp = client
        .post(&url)
        .json(&payload)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp)
}

/// Retrieves an image by constructing a URL with query parameters.
pub async fn get_image(
    client: &Client,
    filename: &str,
    subfolder: &str,
    folder_type: &str,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let url = format!("http://{}/view", SERVER_ADDRESS);
    let params = [
        ("filename", filename),
        ("subfolder", subfolder),
        ("type", folder_type),
    ];
    let bytes = client
        .get(&url)
        .query(&params)
        .send()
        .await?
        .bytes()
        .await?;
    Ok(bytes.to_vec())
}

/// Retrieves the history JSON for a given prompt id.
pub async fn get_history(client: &Client, prompt_id: &str) -> Result<Value, Box<dyn Error>> {
    let url = format!("http://{}/history/{}", SERVER_ADDRESS, prompt_id);
    let resp = client.get(&url).send().await?.json().await?;
    Ok(resp)
}

/// Listens on the websocket until the prompt execution is done,
/// then downloads images from the history.
pub async fn get_images(
    ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    client: &Client,
    prompt: &Value,
    client_id: &str,
    //ctx: &mut TaskContext,
) -> Result<HashMap<String, Vec<Vec<u8>>>, Box<dyn Error>> {
    // Submit the prompt and get the prompt_id.
    let queue_resp = queue_prompt(client, prompt, client_id).await?;
    // dbg!(&queue_resp);
    let prompt_id = queue_resp["prompt_id"]
        .as_str()
        .ok_or("prompt_id not found")?;

    // // Listen for websocket messages until we see one indicating execution is done.

    while let Some(msg) = ws.next().await {
        let msg = msg?;
        if msg.is_text() {
            let text = msg.into_text()?;
            //dbg!(&text);
            let message: Value = serde_json::from_str(&text)?;
            if message["type"] == "executing" {
                let data = &message["data"];
                // When data["node"] is null and the prompt_id matches, execution is done.
                if data["node"].is_null() && data["prompt_id"] == prompt_id {
                    break;
                }
            }
            if message["type"] == "progress" {
                // let data = &message["data"];
                // let node = data["node"].as_str().unwrap();
                // let max = data["max"].as_i64().unwrap();
                // let value = data["value"].as_i64().unwrap();
                // let progress = (value as f64 / max as f64) * 100.0;

                // TODO: synchronize with main thread to see progress, println works, info spans dont,
                // https://github.com/EkardNT/bevy-tokio-tasks#how-to-synchronize-with-the-main-thread
                //println!("Progress: {} - {:4.0}", node, progress);
                // ctx.run_on_main_thread(move |ctx| {
                //     // The inner context gives access to a mutable Bevy World reference.
                //     let _world: &mut World = ctx.world;
                // }).await;
            }
            if message["type"] == "error" {
                let data = &message["data"];
                dbg!("Error: {:?}", data);
            }
        }
    }

    // Get history for the executed prompt.
    let history: Value = get_history(client, prompt_id).await?;
    //dbg!("History: {:?}", &history);
    let history_for_prompt = &history[prompt_id];
    let outputs = history_for_prompt["outputs"]
        .as_object()
        .ok_or("outputs not found")?;

    let mut output_images: HashMap<String, Vec<Vec<u8>>> = HashMap::new();

    // Iterate through each node's output.
    for (node_id, node_output) in outputs.iter() {
        let mut images_output = Vec::new();
        if let Some(output) = node_output.get("images") {
            if let Some(arr) = output.as_array() {
                for image in arr {
                    let filename = image["filename"].as_str().ok_or("filename not found")?;
                    let subfolder = image["subfolder"].as_str().ok_or("subfolder not found")?;
                    let folder_type = image["type"].as_str().ok_or("type not found")?;
                    let image_data = get_image(client, filename, subfolder, folder_type).await?;
                    images_output.push(image_data);
                }
            }
        }
        output_images.insert(node_id.clone(), images_output);
    }

    Ok(output_images)
}

/// Listens on the websocket until the prompt execution is done,
/// then downloads images from the history.
pub async fn get_models(
    ws: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    client: &Client,
    prompt: &Value,
    client_id: &str,
    model_node_id: &str,
    //ctx: &mut TaskContext,
) -> Result<HashMap<String, Vec<Vec<u8>>>, Box<dyn Error>> {
    // Submit the prompt and get the prompt_id.
    let queue_resp = queue_prompt(client, prompt, client_id).await?;
    // dbg!(&queue_resp);
    let prompt_id = queue_resp["prompt_id"]
        .as_str()
        .ok_or("prompt_id not found")?;

    // // Listen for websocket messages until we see one indicating execution is done.

    while let Some(msg) = ws.next().await {
        let msg = msg?;
        if msg.is_text() {
            let text = msg.into_text()?;
            //dbg!(&text);
            let message: Value = serde_json::from_str(&text)?;
            if message["type"] == "executing" {
                let data = &message["data"];
                // When data["node"] is null and the prompt_id matches, execution is done.
                if data["node"].is_null() && data["prompt_id"] == prompt_id {
                    break;
                }
            }
            if message["type"] == "progress" {
                // let data = &message["data"];
                // let node = data["node"].as_str().unwrap();
                // let max = data["max"].as_i64().unwrap();
                // let value = data["value"].as_i64().unwrap();
                // let progress = (value as f64 / max as f64) * 100.0;

                // TODO: synchronize with main thread to see progress, println works, info spans dont,
                // https://github.com/EkardNT/bevy-tokio-tasks#how-to-synchronize-with-the-main-thread
                //println!("Progress: {} - {:4.0}", node, progress);
                // ctx.run_on_main_thread(move |ctx| {
                //     // The inner context gives access to a mutable Bevy World reference.
                //     let _world: &mut World = ctx.world;
                // }).await;
            }
            if message["type"] == "error" {
                let data = &message["data"];
                dbg!("Error: {:?}", data);
            }
        }
    }

    // Get history for the executed prompt.
    let history: Value = get_history(client, prompt_id).await?;
    //dbg!("History: {:?}", &history);
    let history_for_prompt = &history[prompt_id];
    let outputs = history_for_prompt["outputs"]
        .as_object()
        .ok_or("outputs not found")?;

    //dbg!("outputs: {:?}", &outputs);

    let mut output_models: HashMap<String, Vec<Vec<u8>>> = HashMap::new();

    // Iterate through each node's output.
    for (node_id, node_output) in outputs.iter() {
        if node_id != model_node_id {
            continue;
        }
        let mut images_output = Vec::new();
        if let Some(output) = node_output.get("model_file") {
            if let Some(arr) = output.as_array() {
                for file in arr {
                    let filename = file.as_str().ok_or("filename not found")?;
                    // dbg!("filename: {:?}", filename);
                    let model_data = get_image(client, filename, "", "output").await?;
                    // dbg!("model_data: {:?}", &model_data.len());
                    images_output.push(model_data);
                }
            }
        }
        output_models.insert(node_id.clone(), images_output);
    }

    Ok(output_models)
}

pub async fn upload_image(
    client: &Client,
    file_path: String,
    filename: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("http://{}/upload/image", SERVER_ADDRESS);

    // let mut headers = HeaderMap::new();
    // headers.insert(USER_AGENT, HeaderValue::from_static("reqwest"));
    // headers.insert(CONTENT_TYPE, HeaderValue::from_static("image/png"));

    // Open the file asynchronously.
    let file = tokio::fs::File::open(&file_path).await?;
    // Wrap the file in a stream of bytes.
    let file_stream = FramedRead::new(file, BytesCodec::new());

    // Build the multipart part using the stream.
    let file_part = multipart::Part::stream(reqwest::Body::wrap_stream(file_stream))
        .file_name(filename.clone())
        .mime_str("image/png")?;

    // Build the multipart form with additional fields.
    let form = multipart::Form::new()
        .part("image", file_part)
        .text("type", "input".to_string())
        .text("subfolder", "".to_string())
        .text("overwrite", "1".to_string());

    let _resp = client.post(url).multipart(form).send().await?;

    //dbg!(&resp);

    Ok(())
}
