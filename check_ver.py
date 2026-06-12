import re

def main():
    content = open('src-tauri/Cargo.lock').read()
    # Let's search for:
    # name = "tauri"
    # version = "..."
    blocks = content.split('[[package]]')
    for b in blocks:
        if 'name = "tauri"' in b:
            version_match = re.search(r'version = "([^"]+)"', b)
            if version_match:
                print("Tauri version:", version_match.group(1))

if __name__ == '__main__':
    main()
