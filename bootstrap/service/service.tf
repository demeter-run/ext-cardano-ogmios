locals {
  name = "ogmios-${var.network}-${var.ogmios_version}-${var.salt}"
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
      "demeter.run/instance" = local.name
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
