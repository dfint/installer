import ast
import re
import json
from pathlib import Path

project_root_path = Path(__file__).parent.parent
source_path = project_root_path / "src"

translatable_regex = re.compile(r"\Wt!\((\".*?\")")

output = dict()

for file_path in sorted(source_path.rglob("**/*.rs")):
    with open(file_path, encoding="utf-8") as file:
        for i, line in enumerate(file, start=1):
            results = translatable_regex.findall(line)

            for result in results:
                print(f"{file_path.relative_to(project_root_path)}:{i}:", result)
                result = ast.literal_eval(result)
                output[result] = result

if output:
    result_file_path = project_root_path / "locale" / "en.json"
    with open(result_file_path, "w", encoding="utf-8") as result_file:
        result_file.write(json.dumps(output, indent=2, ensure_ascii=False))
