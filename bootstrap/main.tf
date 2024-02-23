locals {
  network_version_combinations = [
    for combo in setproduct(var.networks, var.versions) : {
      network = combo[0]
      version = combo[1]
    }
  ]
}

resource "kubernetes_namespace" "namespace" {
  metadata {
    name = var.namespace
  }
}

module "ogmios_v1_feature" {
  depends_on         = [kubernetes_namespace.namespace]
  source             = "./feature"
  namespace          = var.namespace
  operator_image_tag = var.operator_image_tag
  metrics_delay      = 60
  extension_name     = var.extension_name
}

module "ogmios_v1_proxy" {
  depends_on      = [kubernetes_namespace.namespace]
  source          = "./proxy"
  namespace       = var.namespace
  replicas        = 1
  proxy_image_tag = var.proxy_image_tag
  extension_name  = var.extension_name
}

// mainnet

module "ogmios_configs" {
  depends_on = [kubernetes_namespace.namespace]
  for_each   = { for network in var.networks : "${network}" => network }

  source    = "./configs"
  namespace = var.namespace
  network   = each.value
}

module "ogmios_instances" {
  depends_on = [kubernetes_namespace.namespace, module.ogmios_configs]
  for_each   = var.instances
  source     = "./instance"

  namespace        = var.namespace
  salt             = each.value.salt
  network          = each.value.network
  ogmios_image     = each.value.ogmios_image
  node_private_dns = each.value.node_private_dns
  ogmios_version   = each.value.ogmios_version
  compute_arch     = each.value.compute_arch
}

module "ogmios_services" {
  depends_on = [kubernetes_namespace.namespace]
  for_each   = { for i, nv in local.network_version_combinations : "${nv.network}-${nv.version}" => nv }
  source     = "./service"

  namespace      = var.namespace
  ogmios_version = each.value.version
  network        = each.value.network
}

