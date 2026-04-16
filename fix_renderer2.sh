import re

with open('crates/abrash-raycast/src/renderer/bsp.rs', 'r') as f:
    code = f.read()

# Replace MockTextures and IntegrationBspMap with new constructors
# We will just replace `MockTextures::new()` with `BspTextureCache::new()`
code = code.replace('MockTextures::new()', 'BspTextureCache::new()')
# IntegrationBspMap::new() -> BspMapData::new_integration()
code = code.replace('IntegrationBspMap::new()', 'BspMapData::new_integration()')

# MockTextures is defined inside tests module
code = re.sub(r'    struct MockTextures \{.*?    \}', '', code, flags=re.DOTALL)
code = re.sub(r'    impl MockTextures \{.*?    \}', '', code, flags=re.DOTALL)
code = re.sub(r'    impl BspTextures for MockTextures \{.*?    \}', '', code, flags=re.DOTALL)

# IntegrationBspMap is defined inside tests module
code = re.sub(r'    struct IntegrationBspMap \{.*?    \}', '', code, flags=re.DOTALL)
code = re.sub(r'    impl IntegrationBspMap \{.*?    \}', '', code, flags=re.DOTALL)
code = re.sub(r'    impl BspMap for IntegrationBspMap \{.*?    \}', '', code, flags=re.DOTALL)

# Remove `map: &impl BspMap` instances missing from regex
code = code.replace('&impl BspMap', '&BspMapData')
code = code.replace('&impl BspTextures', '&BspTextureCache')

with open('crates/abrash-raycast/src/renderer/bsp.rs', 'w') as f:
    f.write(code)
