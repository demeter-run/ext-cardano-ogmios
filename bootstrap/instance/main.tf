variable "namespace" {
  type = string
}

variable "ogmios_version" {
  type = string

  validation {
    condition     = contains(["5", "6"], var.ogmios_version)
    error_message = "Invalid version. Allowed values are 5 or 6."
  }
}

variable "ogmios_image" {
  type = string
}

variable "salt" {
  type = string
}

variable "network" {
  type = string
}

variable "node_private_dns" {
  type = string
}

variable "node_affinity" {
  type = object({
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
  })
  default = {}
}

variable "replicas" {
  type    = number
  default = 1
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
      memory : "1Gi"
    }
    requests : {
      cpu : "200m",
      memory : "500Mi"
    }
  }
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
      operator = "Exists"
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

variable "image_pull_secret" {
  type    = string
  default = null
}
