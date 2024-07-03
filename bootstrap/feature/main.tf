locals {
  operator_name = "operator"
  operator_role = "operator"
  operator_port = 9817
  operator_addr = "0.0.0.0:${local.operator_port}"
}

variable "namespace" {
  type = string
}

variable "operator_image_tag" {
  type = string
}

variable "dns_zone" {
  type    = string
  default = "demeter.run"
}

variable "extension_name" {
  type    = string
  default = "ogmios-m1"
}

variable "api_key_salt" {
  type    = string
  default = "ogmios-salt"
}

variable "dcu_per_second" {
  type = map(string)
  default = {
    "mainnet"        = "1"
    "preprod"        = "1"
    "preview"        = "1"
    "vector-testnet" = "1"
  }
}

variable "metrics_delay" {
  type    = number
  default = 30
}

variable "prometheus_url" {
  type    = string
  default = "http://prometheus-operated.demeter-system.svc.cluster.local:9090/api/v1"
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
      cpu : "50m",
      memory : "250Mi"
    }
    requests : {
      cpu : "50m",
      memory : "250Mi"
    }
  }
}
