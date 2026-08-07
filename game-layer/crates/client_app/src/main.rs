use std::io::stdin;

use game_lib::ecs::{MeshHandle, Position, Velocity};
use glam::{Vec2, Vec3};
use mte_macros::vfs_read;

mod config;
mod network;

const REGISTERED: bool = true;

pub struct ResourceManager {}

#[derive(Default, Clone)]
pub struct MeshData {
    primitives: Vec<MeshPrimitive>,
}

#[derive(Default, Clone, Eq, Hash, PartialEq)]
pub struct MeshPrimitive {
    pub material: Material,
    pub offset: u32,
}

#[derive(Default, Clone, Eq, Hash, PartialEq)]
pub struct Material {}

// pub fn render_system(
//     mesh_handles: &Vec<MeshHandle>,
//     _positions: &Vec<Position>,
//     _mesh_data: &Vec<MeshData>,
// ) {
//     // 1. Создаем один плоский массив индексов (одна аллокация)
//     let mut indices: Vec<usize> = (0..mesh_handles.len()).collect();

//     // 2. Сортируем индексы по mesh_handle.
//     // Одинаковые MeshHandle теперь гарантированно стоят рядом.
//     indices.sort_unstable_by_key(|&idx| &mesh_handles[idx]);

//     // 3. Обходим сгруппированные данные с помощью chunk_by (доступен в std)
//     let mut i = 0;
//     while i < indices.len() {
//         let current_mesh_handle = &mesh_handles[indices[i]];

//         // Находим границы группы с одинаковыми мешами
//         let mut group_end = i + 1;
//         while group_end < indices.len() && mesh_handles[indices[group_end]] == *current_mesh_handle
//         {
//             group_end += 1;
//         }

//         // Текущий срез (слайс) индексов для инстансинга
//         let _mesh_group_indices = &indices[i..group_end];

//         // 4. Отправляем в графический конвейер (псевдокод)
//         // render_instanced(current_mesh_handle, mesh_group_indices, positions, mesh_data);

//         i = group_end;
//     }
// }

#[derive(Clone, Copy)]
enum Block {
    Air,
    Dirt,
    Stone,
}

// #[tokio::main]
fn main() {
    println!("{}", size_of::<Block>());

    let chunk = [Block::Dirt; 32 * 32 * 32];

    // lib_renderer::run::run().unwrap();

    // let mut _mesh_handles: Vec<MeshHandle> = Vec::new();
    // let mut _positions: Vec<Position> = Vec::new();
    // let mut _velocities: Vec<Velocity> = Vec::new();

    // let _mesh_data: Vec<MeshData> = Vec::new();

    // for i in 0..10 {
    //     _mesh_handles.push(MeshHandle(0));
    //     _positions.push(Position(Vec3::new(0.0, 0.0, 0.0)));
    //     _velocities.push(Velocity(Vec3::new(0.0, 0.0, 0.0)));
    // }

    // Загрузка уровня:
    // Загрузка всего пака.
    // Макрос сам читает разбиение из .vfs_meta и всё загружает в память
    //

    // vfs_runtime_auto(
    //     include_config!("META_PATH"),
    //     lib_core::vfs::MmapPolicy::AllPacked,
    // )
    // .unwrap();

    // let a = vfs_read!("packs://pack0/assets/textures/fasn-kukuruza.jpg");

    //     // renderer::run::run();
    // if !REGISTERED {
    //     if let Err(e) = NetManager::try_register().await {
    //         println!("Ошибка: {}", e);
    //     } else {
    //         println!("Успешная регистрация");
    //     }
    //
    // };

    // println!("Авторизация...");
    // let auth_data: AuthData = match NetManager::try_login().await {
    //     Ok(d) => d,
    //     Err(e) => {
    //         println!("{}", e);
    //         return;
    //     }
    // };
    // println!("Токен получен");

    // let futures: Vec<_> = auth_data
    //     .servers
    //     .iter()
    //     .map(|s| {
    //         let ping_addr = SocketAddr::new(IpAddr::V4(s.ip), s.port+1);
    //         async move {
    //             let ping_value = NetManager::check_ping(ping_addr).await;
    //             (ping_addr, ping_value)
    //         }
    //     })
    //     .collect();

    // let results: Vec<(SocketAddr, u32)> = join_all(futures).await;

    // let ping_map: HashMap<SocketAddr, u32> = results.into_iter().collect();

    // let game_server_addr = if let Some((addr, min_ping)) =
    //     ping_map.iter().min_by_key(|&(_, ping)| ping)
    //     && *min_ping < 999u32
    // {
    //     addr.clone()
    // } else {
    //     println!("Отсутствует подключение");
    //     return;
    // };

    // println!("Найдены сервера");

    // let (_cmd_udp_tx, cmd_udp_rx) = mpsc::unbounded_channel::<UdpCommand>();
    // let (cmd_ecs_tx, _cmd_ecs_rx) = mpsc::unbounded_channel::<ECSCommand>();

    // NetManager::run_udp(
    //     game_server_addr,
    //     auth_data.access_token,
    //     cmd_udp_rx,
    //     cmd_ecs_tx.clone(),
    // );
}
