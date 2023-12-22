use futures::StreamExt;
use kube::{
    runtime::{controller::Action, watcher::Config as WatcherConfig, Controller},
    Api, Client, CustomResource, ResourceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

use crate::{
    auth::handle_auth,
    build_private_dns_service_name,
    gateway::{handle_http_route, handle_reference_grant},
    Error, Metrics, Network, Result, State,
};

pub static OGMIOS_PORT_FINALIZER: &str = "ogmiosports.demeter.run";

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    kind = "OgmiosPort",
    group = "demeter.run",
    version = "v1alpha1",
    namespaced
)]
#[kube(status = "OgmiosPortStatus")]
#[kube(printcolumn = r#"
        {"name":"Network", "jsonPath": ".spec.network", "type": "string"},
        {"name": "Endpoint URL", "jsonPath": ".status.endpointUrl",  "type": "string"},
        {"name": "Auth Token", "jsonPath": ".status.authToken", "type": "string"}
    "#)]
#[serde(rename_all = "camelCase")]
pub struct OgmiosPortSpec {
    pub network: Network,
}

#[derive(Deserialize, Serialize, Clone, Default, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OgmiosPortStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,
}

struct Context {
    pub client: Client,
    pub metrics: Metrics,
}
impl Context {
    pub fn new(client: Client, metrics: Metrics) -> Self {
        Self { client, metrics }
    }
}

async fn reconcile(crd: Arc<OgmiosPort>, ctx: Arc<Context>) -> Result<Action> {
    let client = ctx.client.clone();
    let namespace = crd.namespace().unwrap();

    let private_dns_service_name = build_private_dns_service_name(&crd.spec.network);
    handle_reference_grant(client.clone(), &namespace, &crd, &private_dns_service_name).await?;
    handle_http_route(client.clone(), &namespace, &crd, &private_dns_service_name).await?;
    handle_auth(client.clone(), &namespace, &crd).await?;

    Ok(Action::await_change())
}

fn error_policy(crd: Arc<OgmiosPort>, err: &Error, ctx: Arc<Context>) -> Action {
    ctx.metrics.reconcile_failure(&crd, err);
    Action::requeue(Duration::from_secs(5))
}

pub async fn run(state: Arc<State>) -> Result<(), Error> {
    let client = Client::try_default().await?;
    let crds = Api::<OgmiosPort>::all(client.clone());
    let ctx = Context::new(client, state.metrics.clone());

    Controller::new(crds, WatcherConfig::default().any_semantic())
        .shutdown_on_signal()
        .run(reconcile, error_policy, Arc::new(ctx))
        .for_each(|_| futures::future::ready(()))
        .await;

    Ok(())
}
