sed -i 's/tangents: Vec::with_capacity(parser.final_vertices.len()),/tangents: Vec::with_capacity(vertex_count),/' src/obj_loader.rs
sed -i '/Ok(Mesh {/i \    let vertex_count = parser.final_vertices.len();' src/obj_loader.rs
