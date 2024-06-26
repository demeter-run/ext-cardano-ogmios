variable "namespace" {
  description = "the namespace where the resources will be created"
}

variable "network" {
  description = "cardano node network"

  validation {
    condition     = contains(["mainnet", "preprod", "preview", "vector-testnet"], var.network)
    error_message = "Invalid network. Allowed values are mainnet, preprod, preview and vector-testnet."
  }
}

variable "ogmios_version" {
  type = string

  validation {
    condition     = contains(["5", "6"], var.ogmios_version)
    error_message = "Invalid version. Allowed values are 5 or 6."
  }
}
