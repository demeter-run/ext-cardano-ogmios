locals {
  port = 1337
}

resource "kubernetes_service_v1" "well_known_service" {
  count = var.well_known_service ? 1 : 0

  metadata {
    name      = var.network
    namespace = var.namespace
  }

  spec {
    selector = {
      "cardano.demeter.run/network"        = var.network
      "cardano.demeter.run/ogmios_version" = var.ogmios_version
    }

    port {
      name        = "api"
      port        = local.port
      target_port = local.port
      protocol    = "TCP"
    }

    type = "ClusterIP"
  }
}
