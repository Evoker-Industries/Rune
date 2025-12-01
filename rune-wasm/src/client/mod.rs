//! Client modules for Rune WASM
//!
//! Provides both remote (WebSocket) and local (offline) container management.

mod local;

pub use local::LocalContainerManager;

use futures::channel::oneshot;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{MessageEvent, WebSocket};

use crate::utils::gloo_timers_sleep;

/// WebSocket-based client for connecting to Rune/Docker daemon
#[wasm_bindgen]
pub struct RuneClient {
    #[wasm_bindgen(skip)]
    pub url: String,
    #[wasm_bindgen(skip)]
    pub ws: Option<WebSocket>,
    #[wasm_bindgen(skip)]
    pub connected: Rc<RefCell<bool>>,
    #[wasm_bindgen(skip)]
    pub pending_requests: Rc<RefCell<HashMap<String, oneshot::Sender<String>>>>,
}

#[wasm_bindgen]
impl RuneClient {
    /// Create a new Rune client
    #[wasm_bindgen(constructor)]
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            ws: None,
            connected: Rc::new(RefCell::new(false)),
            pending_requests: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// Connect to the daemon
    #[wasm_bindgen]
    pub async fn connect(&mut self) -> Result<(), JsValue> {
        let ws = WebSocket::new(&self.url)?;

        let connected = self.connected.clone();
        let pending = self.pending_requests.clone();

        // Set up onopen handler
        let onopen = Closure::wrap(Box::new(move || {
            *connected.borrow_mut() = true;
            web_sys::console::log_1(&"Connected to Rune daemon".into());
        }) as Box<dyn FnMut()>);
        ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        onopen.forget();

        // Set up onmessage handler
        let pending_clone = pending.clone();
        let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                let msg: String = txt.into();
                if let Ok(response) = serde_json::from_str::<serde_json::Value>(&msg) {
                    if let Some(id) = response.get("requestId").and_then(|v| v.as_str()) {
                        if let Some(sender) = pending_clone.borrow_mut().remove(id) {
                            let _ = sender.send(msg);
                        }
                    }
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

        // Set up onerror handler
        let onerror = Closure::wrap(Box::new(move |_e: web_sys::ErrorEvent| {
            web_sys::console::error_1(&"WebSocket error".into());
        }) as Box<dyn FnMut(web_sys::ErrorEvent)>);
        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onerror.forget();

        // Set up onclose handler
        let connected_close = self.connected.clone();
        let onclose = Closure::wrap(Box::new(move |_e: web_sys::CloseEvent| {
            *connected_close.borrow_mut() = false;
            web_sys::console::log_1(&"Disconnected from Rune daemon".into());
        }) as Box<dyn FnMut(web_sys::CloseEvent)>);
        ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
        onclose.forget();

        self.ws = Some(ws);

        // Wait for connection
        let connected_wait = self.connected.clone();
        let mut attempts = 0;
        while !*connected_wait.borrow() && attempts < 50 {
            gloo_timers_sleep(100).await;
            attempts += 1;
        }

        if !*connected_wait.borrow() {
            return Err(JsValue::from_str("Connection timeout"));
        }

        Ok(())
    }

    /// Check if connected
    #[wasm_bindgen(js_name = isConnected)]
    pub fn is_connected(&self) -> bool {
        *self.connected.borrow()
    }

    /// Disconnect from the daemon
    #[wasm_bindgen]
    pub fn disconnect(&mut self) {
        if let Some(ws) = &self.ws {
            let _ = ws.close();
        }
        self.ws = None;
        *self.connected.borrow_mut() = false;
    }

    /// List containers
    #[wasm_bindgen(js_name = listContainers)]
    pub async fn list_containers(&self, all: bool) -> Result<JsValue, JsValue> {
        let endpoint = if all {
            "/containers/json?all=true"
        } else {
            "/containers/json"
        };
        self.http_get(endpoint).await
    }

    /// Get container details
    #[wasm_bindgen(js_name = getContainer)]
    pub async fn get_container(&self, id: &str) -> Result<JsValue, JsValue> {
        let endpoint = format!("/containers/{}/json", id);
        self.http_get(&endpoint).await
    }

    /// Create a container
    #[wasm_bindgen(js_name = createContainer)]
    pub async fn create_container(&self, options_json: &str) -> Result<JsValue, JsValue> {
        self.http_post("/containers/create", options_json).await
    }

    /// Start a container
    #[wasm_bindgen(js_name = startContainer)]
    pub async fn start_container(&self, id: &str) -> Result<JsValue, JsValue> {
        let endpoint = format!("/containers/{}/start", id);
        self.http_post(&endpoint, "{}").await
    }

    /// Stop a container
    #[wasm_bindgen(js_name = stopContainer)]
    pub async fn stop_container(&self, id: &str, timeout: Option<i32>) -> Result<JsValue, JsValue> {
        let endpoint = match timeout {
            Some(t) => format!("/containers/{}/stop?t={}", id, t),
            None => format!("/containers/{}/stop", id),
        };
        self.http_post(&endpoint, "{}").await
    }

    /// Restart a container
    #[wasm_bindgen(js_name = restartContainer)]
    pub async fn restart_container(&self, id: &str) -> Result<JsValue, JsValue> {
        let endpoint = format!("/containers/{}/restart", id);
        self.http_post(&endpoint, "{}").await
    }

    /// Kill a container
    #[wasm_bindgen(js_name = killContainer)]
    pub async fn kill_container(
        &self,
        id: &str,
        signal: Option<String>,
    ) -> Result<JsValue, JsValue> {
        let endpoint = match signal {
            Some(s) => format!("/containers/{}/kill?signal={}", id, s),
            None => format!("/containers/{}/kill", id),
        };
        self.http_post(&endpoint, "{}").await
    }

    /// Remove a container
    #[wasm_bindgen(js_name = removeContainer)]
    pub async fn remove_container(&self, id: &str, force: bool) -> Result<JsValue, JsValue> {
        let endpoint = if force {
            format!("/containers/{}?force=true", id)
        } else {
            format!("/containers/{}", id)
        };
        self.http_delete(&endpoint).await
    }

    /// Get container logs
    #[wasm_bindgen(js_name = getContainerLogs)]
    pub async fn get_container_logs(
        &self,
        id: &str,
        tail: Option<i32>,
    ) -> Result<JsValue, JsValue> {
        let endpoint = match tail {
            Some(n) => format!("/containers/{}/logs?stdout=true&stderr=true&tail={}", id, n),
            None => format!("/containers/{}/logs?stdout=true&stderr=true", id),
        };
        self.http_get(&endpoint).await
    }

    /// List images
    #[wasm_bindgen(js_name = listImages)]
    pub async fn list_images(&self) -> Result<JsValue, JsValue> {
        self.http_get("/images/json").await
    }

    /// Get image details
    #[wasm_bindgen(js_name = getImage)]
    pub async fn get_image(&self, id: &str) -> Result<JsValue, JsValue> {
        let endpoint = format!("/images/{}/json", id);
        self.http_get(&endpoint).await
    }

    /// Remove an image
    #[wasm_bindgen(js_name = removeImage)]
    pub async fn remove_image(&self, id: &str, force: bool) -> Result<JsValue, JsValue> {
        let endpoint = if force {
            format!("/images/{}?force=true", id)
        } else {
            format!("/images/{}", id)
        };
        self.http_delete(&endpoint).await
    }

    /// List networks
    #[wasm_bindgen(js_name = listNetworks)]
    pub async fn list_networks(&self) -> Result<JsValue, JsValue> {
        self.http_get("/networks").await
    }

    /// Create a network
    #[wasm_bindgen(js_name = createNetwork)]
    pub async fn create_network(&self, options_json: &str) -> Result<JsValue, JsValue> {
        self.http_post("/networks/create", options_json).await
    }

    /// Remove a network
    #[wasm_bindgen(js_name = removeNetwork)]
    pub async fn remove_network(&self, id: &str) -> Result<JsValue, JsValue> {
        let endpoint = format!("/networks/{}", id);
        self.http_delete(&endpoint).await
    }

    /// List volumes
    #[wasm_bindgen(js_name = listVolumes)]
    pub async fn list_volumes(&self) -> Result<JsValue, JsValue> {
        self.http_get("/volumes").await
    }

    /// Create a volume
    #[wasm_bindgen(js_name = createVolume)]
    pub async fn create_volume(&self, options_json: &str) -> Result<JsValue, JsValue> {
        self.http_post("/volumes/create", options_json).await
    }

    /// Remove a volume
    #[wasm_bindgen(js_name = removeVolume)]
    pub async fn remove_volume(&self, name: &str) -> Result<JsValue, JsValue> {
        let endpoint = format!("/volumes/{}", name);
        self.http_delete(&endpoint).await
    }

    /// Get system info
    #[wasm_bindgen(js_name = getInfo)]
    pub async fn get_info(&self) -> Result<JsValue, JsValue> {
        self.http_get("/info").await
    }

    /// Get version
    #[wasm_bindgen(js_name = getVersion)]
    pub async fn get_version(&self) -> Result<JsValue, JsValue> {
        self.http_get("/version").await
    }

    /// Ping the daemon
    #[wasm_bindgen]
    pub async fn ping(&self) -> Result<JsValue, JsValue> {
        self.http_get("/_ping").await
    }

    // Internal HTTP methods
    async fn http_get(&self, endpoint: &str) -> Result<JsValue, JsValue> {
        let url = format!(
            "{}{}",
            self.url
                .replace("ws://", "http://")
                .replace("wss://", "https://"),
            endpoint
        );

        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
        let resp_value = JsFuture::from(window.fetch_with_str(&url)).await?;
        let resp: web_sys::Response = resp_value.dyn_into()?;
        let json = JsFuture::from(resp.json()?).await?;
        Ok(json)
    }

    async fn http_post(&self, endpoint: &str, body: &str) -> Result<JsValue, JsValue> {
        let url = format!(
            "{}{}",
            self.url
                .replace("ws://", "http://")
                .replace("wss://", "https://"),
            endpoint
        );

        let mut opts = web_sys::RequestInit::new();
        opts.method("POST");
        opts.body(Some(&JsValue::from_str(body)));

        let request = web_sys::Request::new_with_str_and_init(&url, &opts)?;
        request.headers().set("Content-Type", "application/json")?;

        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
        let resp: web_sys::Response = resp_value.dyn_into()?;
        let json = JsFuture::from(resp.json()?).await?;
        Ok(json)
    }

    async fn http_delete(&self, endpoint: &str) -> Result<JsValue, JsValue> {
        let url = format!(
            "{}{}",
            self.url
                .replace("ws://", "http://")
                .replace("wss://", "https://"),
            endpoint
        );

        let mut opts = web_sys::RequestInit::new();
        opts.method("DELETE");

        let request = web_sys::Request::new_with_str_and_init(&url, &opts)?;

        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
        let resp: web_sys::Response = resp_value.dyn_into()?;
        let json = JsFuture::from(resp.json()?).await?;
        Ok(json)
    }
}
