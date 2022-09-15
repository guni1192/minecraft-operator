use anyhow::Result;
use kube::{CustomResource, CustomResourceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Serialize, Deserialize, Default, Debug, PartialEq, Clone, JsonSchema)]
#[kube(
    group = "guni.dev",
    version = "v1",
    kind = "Minecraft",
    shortname = "mc",
    namespaced,
    derive = "PartialEq",
    derive = "Default"
)]
pub struct MinecraftSpec {
    name: String,
}

pub fn generate_crds() -> Result<()> {
    let crd = serde_yaml::to_string(&Minecraft::crd())?;
    println!("{}", crd);
    Ok(())
}
