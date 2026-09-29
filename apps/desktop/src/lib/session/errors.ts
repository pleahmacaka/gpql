import * as m from "$lib/paraglide/messages"

const HINTS: [needle: string, hint: () => string][] = [
  ["password missing", m.error_needs_password],
  ["os error 10061", m.error_no_listener],
  ["connection refused", m.error_no_listener],
  ["os error 10060", m.error_no_answer],
  ["os error 11004", m.error_ipv6_only],
  ["enoidentifier", m.error_needs_tenant],
  ["failed to lookup address", m.error_bad_host],
  ["can not tell whether that statement writes", m.error_unsure_write],
  ["nothing was committed", m.error_tx_failed],
  ["turn on manual commit", m.error_tx_needs_manual],
  ["commits the open transaction by itself", m.error_implicit_commit],
  ["does not match the one on line", m.error_host_key],
  ["tailscale is not installed", m.error_no_tailscale],
]

export function friendly(text: string) {
  return text.includes("gpql.no_model") ? m.no_model() : text
}

export function hint(text: string) {
  const lower = text.toLowerCase()
  const found = HINTS.find(([needle]) => lower.includes(needle))

  return found ? found[1]() : ""
}
