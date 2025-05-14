locals {
  name = var.name != null ? var.name : "proxy-${var.network}"
  role = "proxy-${var.network}"

  prometheus_port = 9187
  prometheus_addr = "0.0.0.0:${local.prometheus_port}"
  proxy_port      = 8080
  proxy_addr      = "0.0.0.0:${local.proxy_port}"
  # proxy_labels = var.environment != null ? { role = local.role, environment = var.environment } : { role = local.role }
  proxy_labels = var.environment != null ? { role = "${local.role}-${var.environment}" } : { role = local.role }
}

variable "name" {
  type    = string
  default = null
}

// blue - green
variable "environment" {
  default = null
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

variable "dns_zone" {
  type    = string
  default = "demeter.run"
}

variable "cluster_issuer" {
  type    = string
  default = "letsencrypt"
}

variable "cloud_provider" {
  type    = string
  default = "aws"
}

variable "healthcheck_port" {
  type    = number
  default = null
}
