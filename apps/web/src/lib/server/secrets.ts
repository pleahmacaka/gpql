const SECRET_PARAMS = [
  "token",
  "pass",
  "pwd",
  "secret",
  "key",
  "auth",
  "sig",
  "credential",
]

function queryStart(part: string) {
  let at = part.indexOf("?")

  while (at !== -1) {
    const rest = part.slice(at + 1)
    const equals = rest.indexOf("=")
    const sign = rest.indexOf("@")

    if (equals !== -1 && (sign === -1 || equals < sign)) {
      return at
    }

    at = part.indexOf("?", at + 1)
  }

  return part.length
}

function withoutPasswords(text: string) {
  return text
    .split("://")
    .map((part, index) => {
      const head = part.slice(0, queryStart(part))
      const sign = head.lastIndexOf("@")

      if (index === 0 || sign === -1 || !head.slice(0, sign).includes(":")) {
        return part
      }

      return part.slice(sign + 1)
    })
    .join("://")
}

function withoutSecretParams(text: string) {
  const at = text.indexOf("?")

  if (at === -1) {
    return text
  }

  const params = new URLSearchParams(text.slice(at + 1))
  const secret = [...params.keys()].filter(name =>
    SECRET_PARAMS.some(word => name.toLowerCase().includes(word)),
  )

  if (secret.length === 0) {
    return text
  }

  for (const name of secret) {
    params.delete(name)
  }

  const rest = params.toString()

  return rest === "" ? text.slice(0, at) : `${text.slice(0, at)}?${rest}`
}

// mirrors stripSecrets in the desktop app's commands.ts, which keys recents
export function stripSecrets(text: string) {
  return withoutSecretParams(withoutPasswords(text))
}

export function carriesSecret(text: string) {
  return stripSecrets(text) !== text
}
