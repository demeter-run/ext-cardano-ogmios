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

variable "node_private_dns" {
  type = string
}

variable "network" {
  type = string
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


variable "compute_arch" {
  type = string
}