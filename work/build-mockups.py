"""Assemble the self-contained review mock; no frontend dependencies required."""
from pathlib import Path
import re

root = Path(__file__).resolve().parent
source = root / "activity-hierarchy.html"
markup = source.read_text()
markup = re.sub(r'<!-- JOURNEY START -->.*?<!-- JOURNEY END -->\s*', '', markup, flags=re.S)
journey = root.joinpath("journey-ui.html").read_text()
journey += '\n' + root.joinpath("wipe-ui.html").read_text()
journey += '\n<script>\n' + root.joinpath("wipe-interactions.js").read_text() + '\n</script>\n'
journey += '\n' + root.joinpath("drawer-ui.html").read_text()
journey += '\n<script>\n' + root.joinpath("drawer-interactions.js").read_text() + '\n</script>\n'
journey += '\n' + root.joinpath("comparison-ui.html").read_text()
journey += '\n<script>\n' + root.joinpath("comparison-interactions.js").read_text() + '\n</script>\n'
journey += '\n' + root.joinpath("examples-ui.html").read_text()
journey += '\n<script>\n' + root.joinpath("examples-interactions.js").read_text() + '\n</script>\n'
journey += '\n<script>\n' + root.joinpath("journey-interactions.js").read_text() + '\n</script>\n'
markup = markup.replace('<script id="wh-preview-script">', '<!-- JOURNEY START -->\n' + journey + '<!-- JOURNEY END -->\n<script id="wh-preview-script">')
markup = re.sub(r'<script id="wh-preview-script">.*?</script>', lambda _: '<script id="wh-preview-script">\n' + root.joinpath("mockup-interactions.js").read_text() + '</script>', markup, flags=re.S)
source.write_text(markup)
output = root / "mockups.html"
head = output.read_text().split('<body>')[0]
body = root.joinpath("preview-controls.html").read_text().replace('<!--WH_PREVIEW_CONTENT-->', markup)
output.write_text(head + '<body>\n' + body + '\n</body></html>')
print(output)
