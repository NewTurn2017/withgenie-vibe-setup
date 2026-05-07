import { readFileSync } from 'node:fs';

const files = [
  'docs/recipe-matrix.md',
  'docs/health-report-schema.md',
  'schemas/health-report.schema.json',
];

const failures = [];
for (const file of files) {
  const text = readFileSync(file, 'utf8');
  if (/TODO|TBD/.test(text)) failures.push(`${file}: TODO/TBD marker found`);
}

const recipe = readFileSync('docs/recipe-matrix.md', 'utf8');
for (const line of recipe.split('\n')) {
  if (line.includes('vercel login --github') && !/금지|Forbidden|forbidden|deprecated|lint/.test(line)) {
    failures.push('deprecated Vercel flag outside forbidden context');
  }
  if (line.includes('npm install -g supabase') && !/금지|Forbidden|forbidden|지원되지|unsupported|lint/.test(line)) {
    failures.push('unsupported Supabase global npm install outside forbidden/support warning context');
  }
}

JSON.parse(readFileSync('schemas/health-report.schema.json', 'utf8'));

if (failures.length) {
  console.error(failures.join('\n'));
  process.exit(1);
}
console.log('문서 검증 통과');
