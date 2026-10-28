#!/usr/bin/osascript -l JavaScript
// Alfred script filter for Napkin: evaluates the query with `napkin --json` and returns one item.
// Enter copies the result, Cmd+Enter copies "expression = result", Alt+Enter copies the expression.
ObjC.import('stdlib')

const app = Application.currentApplication()
app.includeStandardAdditions = true

// Where `napkin` usually lives; Alfred runs scripts with a minimal PATH. The workflow variable
// `napkin_path` overrides the search.
const CANDIDATES = ['~/.cargo/bin/napkin', '/opt/homebrew/bin/napkin', '/usr/local/bin/napkin']

function env(name) {
  const value = $.getenv(name)
  return typeof value === 'string' ? value : ''
}

function expandHome(path) {
  return path.startsWith('~/') ? env('HOME') + path.slice(1) : path
}

function findNapkin() {
  const configured = env('napkin_path').trim()
  const candidates = configured ? [configured] : CANDIDATES
  const files = $.NSFileManager.defaultManager
  return candidates.map(expandHome).find((path) => files.isExecutableFileAtPath(path)) || null
}

function shellQuote(text) {
  return "'" + text.replace(/'/g, "'\\''") + "'"
}

function items(list) {
  return JSON.stringify({ items: list })
}

function message(title, subtitle) {
  return items([{ title, subtitle, valid: false }])
}

function run(argv) {
  const query = (argv[0] || '').trim()
  if (!query) {
    return message('Napkin', 'Type an expression, e.g. 20 inches in cm')
  }
  const napkin = findNapkin()
  if (!napkin) {
    return message('napkin not found', 'Install the CLI (cargo install) or set the napkin_path workflow variable')
  }

  let line
  try {
    // `napkin` exits 1 when the expression fails; the JSON on stdout still says why.
    const output = app.doShellScript(`${shellQuote(napkin)} --json -- ${shellQuote(query)} 2>/dev/null || true`)
    line = JSON.parse(output)
  } catch (error) {
    return message('Napkin failed', String(error))
  }

  if (!line.result) {
    return message('…', line.error || 'No result')
  }
  const result = line.result.text
  const workings = `${query} = ${result}`
  return items([
    {
      uid: 'napkin-result',
      title: result,
      subtitle: `${query}  ·  Enter copies the result`,
      arg: result,
      text: { copy: result, largetype: workings },
      mods: {
        cmd: { subtitle: `Copy “${workings}”`, arg: workings },
        alt: { subtitle: 'Copy the expression', arg: query },
      },
    },
  ])
}
