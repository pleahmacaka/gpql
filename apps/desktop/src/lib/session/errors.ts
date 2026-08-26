import * as m from "$lib/paraglide/messages"

const known: Record<string, () => string> = {
  "gpql.needs_password": m.error_needs_password,
  "gpql.no_listener": m.error_no_listener,
  "gpql.no_answer": m.error_no_answer,
  "gpql.bad_host": m.error_bad_host,
  "gpql.ipv6_only": m.error_ipv6_only,
  "gpql.needs_tenant": m.error_needs_tenant,
  "gpql.login_gone": m.error_login_gone,
  "gpql.no_model": m.no_model,
}

export function friendly(text: string) {
  const key = Object.keys(known).find(code => text.includes(code))

  return key ? known[key]() : text
}
