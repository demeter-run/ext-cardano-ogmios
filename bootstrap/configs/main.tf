variable "network" {
  description = "cardano node network"
}

variable "namespace" {
  description = "the namespace where the resources will be created"
}

resource "kubernetes_config_map" "node-config" {
  metadata {
    namespace = var.namespace
    name      = "configs-${var.network}"
  }

  data = {
    "config.json" = "${file("${path.module}/${var.network}/config.json")}"
  }
}

resource "kubernetes_config_map" "genesis" {
  for_each = var.network == "vector-testnet" ? toset(["vector-testnet"]) : toset([])

  metadata {
    namespace = var.namespace
    name      = "genesis-${var.network}"
  }

  data = {
    "alonzo.json"  = "${file("${path.module}/${var.network}/alonzo.json")}"
    "byron.json"   = "${file("${path.module}/${var.network}/byron.json")}"
    "conway.json"  = "${file("${path.module}/${var.network}/conway.json")}"
    "shelley.json" = "${file("${path.module}/${var.network}/shelley.json")}"
  }
}

output "cm_name" {
  value = "configs-${var.network}"
}
