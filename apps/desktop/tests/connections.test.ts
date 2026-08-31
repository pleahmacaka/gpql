import { expect, test } from "bun:test"

import {
  type Connection,
  foldersOf,
  reorder,
  sectionsOf,
  shiftBy,
  toTop,
} from "../src/lib/session/connections"

const entry = (url: string, extra: Partial<Connection> = {}): Connection => ({
  url,
  kind: "postgres",
  label: url,
  alias: null,
  detail: "127.0.0.1:5432",
  folder: null,
  rank: 0,
  tunnelled: 0,
  openedAt: 0,
  ...extra,
})

test("dragging a connection down lands it where it was dropped", () => {
  expect(reorder(["a", "b", "c", "d"], "a", "c")).toEqual(["b", "c", "a", "d"])
})

test("dragging a connection up lands it where it was dropped", () => {
  expect(reorder(["a", "b", "c", "d"], "d", "b")).toEqual(["a", "d", "b", "c"])
})

test("dropping a connection on itself changes nothing", () => {
  expect(reorder(["a", "b"], "a", "a")).toEqual(["a", "b"])
})

test("a drop involving an unknown connection changes nothing", () => {
  expect(reorder(["a", "b"], "a", "zz")).toEqual(["a", "b"])
  expect(reorder(["a", "b"], "zz", "a")).toEqual(["a", "b"])
})

test("folders are listed once, in the order they first appear", () => {
  const listed = foldersOf([
    entry("a", { folder: "prod" }),
    entry("b", { folder: "dev" }),
    entry("c", { folder: "prod" }),
    entry("d"),
  ])

  expect(listed).toEqual(["prod", "dev"])
})

test("a folder named only with spaces does not count as a group", () => {
  expect(foldersOf([entry("a", { folder: "   " })])).toEqual([])
})

test("grouped connections come before the loose ones", () => {
  const sections = sectionsOf([
    entry("a", { folder: "prod" }),
    entry("b"),
    entry("c", { folder: "prod" }),
  ])

  expect(sections.map(section => section.folder)).toEqual(["prod", null])
  expect(sections[0].entries.map(item => item.url)).toEqual(["a", "c"])
  expect(sections[1].entries.map(item => item.url)).toEqual(["b"])
})

test("with nothing grouped there is a single unnamed section", () => {
  const sections = sectionsOf([entry("a"), entry("b")])

  expect(sections).toHaveLength(1)
  expect(sections[0].folder).toBeNull()
})

test("with everything grouped no loose section is added", () => {
  const sections = sectionsOf([entry("a", { folder: "prod" })])

  expect(sections.map(section => section.folder)).toEqual(["prod"])
})

test("no connections means no sections", () => {
  expect(sectionsOf([])).toEqual([])
})

test("moving a connection up swaps it with the one above", () => {
  expect(shiftBy(["a", "b", "c"], "c", -1)).toEqual(["a", "c", "b"])
})

test("moving a connection down swaps it with the one below", () => {
  expect(shiftBy(["a", "b", "c"], "a", 1)).toEqual(["b", "a", "c"])
})

test("moving past either end changes nothing", () => {
  expect(shiftBy(["a", "b"], "a", -1)).toEqual(["a", "b"])
  expect(shiftBy(["a", "b"], "b", 1)).toEqual(["a", "b"])
  expect(shiftBy(["a", "b"], "zz", 1)).toEqual(["a", "b"])
})

test("sending a connection to the top puts it first", () => {
  expect(toTop(["a", "b", "c"], "c")).toEqual(["c", "a", "b"])
  expect(toTop(["a", "b"], "a")).toEqual(["a", "b"])
  expect(toTop([], "a")).toEqual([])
})
