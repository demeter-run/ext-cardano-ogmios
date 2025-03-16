variable "namespace" {
  type = string
}

variable "dns_zone" {
  type    = string
  default = "demeter.run"
}

variable "cluster_issuer" {
  type    = string
  default = "letsencrypt"
}

variable "extension_name" {
  type    = string
  default = "ogmios-m1"
}

variable "cloud_provider" {
  type    = string
  default = "aws"
}

variable "networks" {
  type    = list(string)
  default = ["mainnet", "preprod", "preview", "vector-testnet"]
}

variable "versions" {
  type    = list(string)
  default = ["5", "6"]
}

variable "o11y_datasource_uid" {
  type    = string
  default = null
}

// operator settings

variable "operator_image_tag" {
  type = string
}

variable "api_key_salt" {
  type    = string
  default = "ogmios-salt"
}

variable "dcu_per_frame" {
  type = map(string)
  default = {
    "mainnet"        = "10"
    "preprod"        = "5"
    "preview"        = "5"
    "vector-testnet" = "5"
  }
}

variable "metrics_delay" {
  type    = number
  default = 60
}

variable "prometheus_url" {
  type    = string
  default = "http://prometheus-operated.demeter-system.svc.cluster.local:9090/api/v1"
}

variable "operator_resources" {
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

// proxy green settings
variable "proxy_green_name" {
  type    = string
  default = "proxy-green"
}

variable "proxy_green_image_tag" {
  type = string
}

variable "proxy_green_replicas" {
  type    = number
  default = 1
}

variable "proxy_green_extra_annotations" {
  type    = map(string)
  default = {}
}

variable "proxy_green_environment" {
  type    = string
  default = "green"
}

// proxy blue settings
variable "proxy_blue_name" {
  type    = string
  default = "proxy"
}

variable "proxy_blue_image_tag" {
  type = string
}

variable "proxy_blue_replicas" {
  type    = number
  default = 1
}

variable "proxy_blue_extra_annotations" {
  type    = map(string)
  default = {}
}

variable "proxy_blue_environment" {
  type    = string
  default = null
}

variable "proxy_resources" {
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

variable "instances" {
  type = map(object({
    salt              = string
    network           = string
    ogmios_image      = string
    node_private_dns  = string
    ogmios_version    = string
    replicas          = number
    image_pull_secret = optional(string)
    resources = optional(object({
      limits = object({
        cpu    = string
        memory = string
      })
      requests = object({
        cpu    = string
        memory = string
      })
    }))
    tolerations = optional(list(object({
      effect   = string
      key      = string
      operator = string
      value    = optional(string)
    })))
    node_affinity = optional(object({
      required_during_scheduling_ignored_during_execution = optional(
        object({
          node_selector_term = optional(
            list(object({
              match_expressions = optional(
                list(object({
                  key      = string
                  operator = string
                  values   = list(string)
                })), []
              )
            })), []
          )
        }), {}
      )
      preferred_during_scheduling_ignored_during_execution = optional(
        list(object({
          weight = number
          preference = object({
            match_expressions = optional(
              list(object({
                key      = string
                operator = string
                values   = list(string)
              })), []
            )
            match_fields = optional(
              list(object({
                key      = string
                operator = string
                values   = list(string)
              })), []
            )
          })
        })), []
      )
    }))
  }))
}
