use std::fs;

pub fn generate_cloud_init(public_url: &str) -> String {
    let template = fs::read_to_string("../artcraft-cloud/deploy/cloud-init/artcraft-cloud.yaml")
        .expect("Failed to read cloud-init template");
    
    template.replace("${PUBLIC_URL}", public_url)
}
