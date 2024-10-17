use fdt_parser::Fdt;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .init();

    let bytes = include_bytes!("../dtb/bcm2711-rpi-4-b.dtb");

    let fdt = Fdt::from_bytes(bytes).unwrap();
    println!("version: {}", fdt.version());
    for region in fdt.reserved_memory_regions() {
        println!("region: {:?}", region);
    }
    let mut i = 0;
    for node in fdt.all_nodes() {
        // if i > 20 {
        //     break;
        // }
        let space = " ".repeat((node.level - 1) * 4);
        println!("{}{}", space, node.name());

        if let Some(cap) = node.compatible() {
            println!("{} -compatible: ", space);
            for cap in cap {
                println!("{}     {:?}", space, cap);
            }
        }

        // if let Some(s) = node.node_ranges() {
        //     println!("{} -node range: ", space);
        //     for range in s.iter() {
        //         println!("{}     {:?}", space, range);
        //     }
        // }
        // println!("{} -range: ", space);
        // for range in node.ranges() {
        //     println!("{}     {:?}", space, range);
        // }

        if let Some(reg) = node.reg() {
            println!("{} - reg: ", space);
            for cell in reg {
                println!("{}     {:?}", space, cell);
            }
        }
        i += 1;
        // for prop in node.propertys() {
        //     println!("{} - {}", space, prop.name);
        // }
    }
}