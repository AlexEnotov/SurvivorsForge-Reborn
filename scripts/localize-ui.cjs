// Mark catalogued Slint literals for bundled translations. Safe to run again after adding entries.
const fs = require('node:fs');
const path = require('node:path');
const catalog = JSON.parse(fs.readFileSync('translations/catalog.json', 'utf8'));
const aliases = new Map([
  ['Запустить лаунчер', 'Launch launcher'], ['Запустить без лаунчера', 'Launch without launcher'],
  ['Запущено', 'Running'], ['Включён', 'Enabled'], ['Выключен', 'Disabled'],
  ['Частично', 'Partly enabled'], ['Включить ', 'Enable '], [' для всех персонажей', ' for all characters'],
  ...['Switches apply to all existing characters. Close the game before making changes.',
    'To switch addons, first enter the game with a character, close it, then check for updates.']
    .map(source => [catalog[source][0], source]),
]);
const files = ['ui', 'ui/components'].flatMap(dir => fs.readdirSync(dir)
  .filter(file => file.endsWith('.slint')).map(file => path.join(dir, file)));
for (const file of files) {
  const input = fs.readFileSync(file, 'utf8');
  const output = input.replace(/"(?:[^"\\]|\\.)*"/g, (literal, offset) => {
    if (input.slice(Math.max(0, offset - 4), offset) === '@tr(') return literal;
    let source;
    try { source = JSON.parse(literal); } catch { return literal; }
    source = aliases.get(source) ?? source;
    return Object.hasOwn(catalog, source) ? `@tr(${JSON.stringify(source)})` : literal;
  });
  if (output !== input) fs.writeFileSync(file, output);
}
