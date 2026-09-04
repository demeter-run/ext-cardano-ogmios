locals {
  config_map_name = "${var.environment}-proxy-config"

  # The tiers a customer can be on by their own action. Their connection caps
  # are per proxy replica, not per deployment.
  self_serve_tiers = [
    {
      "name"            = "0",
      "max_connections" = 2
      "rates" = [
        {
          "interval" = "1m",
          "limit"    = 500
        }
      ]
    },
    {
      "name"            = "1",
      "max_connections" = 5
      "rates" = [
        {
          "interval" = "1m",
          "limit"    = 500
        }
      ]
    },
    {
      "name"            = "2",
      "max_connections" = 250
      "rates" = [
        {
          "interval" = "1m",
          "limit"    = 500
        }
      ]
    },
    {
      "name"            = "3",
      "max_connections" = 450
      "rates" = [
        {
          "interval" = "1m",
          "limit"    = 1500
        }
      ]
    }
  ]

  # Internal enterprise tier. It is assigned only through the backoffice and is
  # never offered as a customer choice, so it carries no consumer identity here
  # -- a port is on it because its CRD says so, like every other tier.
  #
  # Its message rate is tier 3's by reference rather than by copy: the two are
  # meant to stay equal, and a copy would silently diverge the next time tier 3
  # is retuned. Only the connection cap differs, and it stays where tier 3's is
  # today while the self-serve caps come down.
  enterprise_tier = {
    "name"            = "4",
    "max_connections" = 450
    "rates"           = one([for tier in local.self_serve_tiers : tier.rates if tier.name == "3"])
  }

  tiers = concat(local.self_serve_tiers, [local.enterprise_tier])
}

resource "kubernetes_config_map" "proxy" {
  metadata {
    namespace = var.namespace
    name      = local.config_map_name
  }

  data = {
    "tiers.toml" = "${templatefile("${path.module}/proxy-config.toml.tftpl", { tiers = local.tiers })}"
  }
}
