function quoteName(name: string, kind: string) {
  if (kind === "mysql") {
    return `\`${name.replaceAll("`", "``")}\``
  }

  return `"${name.replaceAll('"', '""')}"`
}

export function selectFrom(name: string, kind: string, dialect: string) {
  if (dialect === "cypher") {
    return `match (n:\`${name.replaceAll("`", "``")}\`) return n limit 100`
  }

  return `select * from ${quoteName(name, kind)} limit 100`
}
