locals {
  name = "proxy-${var.network}-${var.environment}"
  role = "proxy-${var.network}-${var.environment}"

  prometheus_port = 9187
  prometheus_addr = "0.0.0.0:${local.prometheus_port}"
  proxy_port      = 8080
  proxy_addr      = "0.0.0.0:${local.proxy_port}"
  proxy_labels    = { role = "${local.role}" }

  by_version = flatten([
    for version in var.versions : "*.${var.network}-v${version}.${var.extension_name}.${var.dns_zone}"
  ])

  # Add the extra URL to the list of generated URLs
  dns_names        = var.dns_names != null ? var.dns_names : concat(local.by_version, ["*.${var.extension_name}.${var.dns_zone}"])
  cert_secret_name = var.environment != null ? "${var.extension_name}-${var.environment}-${var.network}-wildcard-tls" : "${var.extension_name}-${var.network}-wildcard-tls"
}

// blue - green
variable "environment" {
  type = string
}

variable "extra_annotations" {
  description = "Extra annotations to add to the proxy services"
  type        = map(string)
  default     = {}
}

variable "namespace" {
  type = string
}

variable "replicas" {
  type    = number
  default = 1
}

variable "proxy_image_tag" {
  type = string
}

variable "resources" {
  type = object({
    limits = object({
      cpu    = string
      memory = string
    })
    requests = object({
      cpu    = string
      memory = string
    })
  })
  default = {
    limits : {
      cpu : "2",
      memory : "250Mi"
    }
    requests : {
      cpu : "100m",
      memory : "250Mi"
    }
  }
}

variable "ogmios_port" {
  type    = number
  default = 1337
}


variable "extension_name" {
  type = string
}

variable "network" {
  type = string
}

variable "versions" {
  type    = list(string)
  default = ["5", "6"]
}

variable "dns_names" {
  type    = list(string)
  default = null
}

variable "dns_zone" {
  type    = string
  default = "demeter.run"
}

variable "cluster_issuer" {
  type    = string
  default = "letsencrypt-dns01"
}

variable "cloud_provider" {
  type    = string
  default = "aws"
}

variable "healthcheck_port" {
  type    = number
  default = null
}

variable "tolerations" {
  description = "List of tolerations for the instance"
  type = list(object({
    effect   = string
    key      = string
    operator = string
    value    = optional(string)
  }))
  default = [
    {
      effect   = "NoSchedule"
      key      = "demeter.run/compute-profile"
      operator = "Equal"
      value    = "general-purpose"
    },
    {
      effect   = "NoSchedule"
      key      = "demeter.run/compute-arch"
      operator = "Equal"
      value    = "x86"
    },
    {
      effect   = "NoSchedule"
      key      = "demeter.run/availability-sla"
      operator = "Equal"
      value    = "consistent"
    }
  ]
}
