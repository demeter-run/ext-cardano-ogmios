

locals {
  by_version = flatten([
    for version in var.versions : "*.${var.network}-v${version}.${var.extension_name}.${var.dns_zone}"
  ])

  # Add the extra URL to the list of generated URLs
  dns_names        = concat(local.by_version, ["*.${var.extension_name}.${var.dns_zone}"])
  cert_secret_name = var.environment != null ? "${var.extension_name}-${var.environment}-${var.network}-wildcard-tls" : "${var.extension_name}-${var.network}-wildcard-tls"
}

resource "kubernetes_manifest" "certificate_cluster_wildcard_tls" {
  manifest = {
    "apiVersion" = "cert-manager.io/v1"
    "kind"       = "Certificate"
    "metadata" = {
      "name"      = local.cert_secret_name
      "namespace" = var.namespace
    }
    "spec" = {
      "dnsNames" = local.dns_names

      "issuerRef" = {
        "kind" = "ClusterIssuer"
        "name" = var.cluster_issuer
      }
      "secretName" = local.cert_secret_name
    }
  }
}
