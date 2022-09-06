use k8s_openapi::api::core::v1::Pod;
use kube::api::{Api, ListParams};
use kube::client::Client;
use kube::ResourceExt;
// use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::default_namespaced(client);

    let list_params = ListParams::default();
    let pods = pods.list(&list_params).await?;

    for pod in pods {
        println!("pod/{}", pod.name_any());
    }

    Ok(())
}
