variable "namespace" {
  type = string
}

variable "dns_zone" {
  type    = string
  default = "demeter.run"
}

variable "cluster_issuer" {
  type    = string
  default = "letsencrypt-dns01"
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

variable "proxy_green_image_tag" {
  type = string
}

variable "proxy_green_replicas" {
  type    = number
  default = 1
}

variable "proxy_green_extra_annotations_by_network" {
  description = <<EOT
A map where keys are network names (only those defined in the "networks" variable)
and values are maps of extra annotations for the blue proxy service specific
to that network.
EOT
  type        = map(map(string))
  default     = {}
}

variable "proxy_green_environment" {
  type    = string
  default = "green"
}

variable "proxy_blue_image_tag" {
  type = string
}

variable "proxy_blue_replicas" {
  type    = number
  default = 1
}

variable "proxy_blue_extra_annotations_by_network" {
  description = <<EOT
A map where keys are network names (only those defined in the "networks" variable)
and values are maps of extra annotations for the blue proxy service specific
to that network.
EOT
  type        = map(map(string))
  default     = {}
}

variable "proxy_blue_environment" {
  type    = string
  default = "blue"
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

variable "proxy_tolerations" {
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
