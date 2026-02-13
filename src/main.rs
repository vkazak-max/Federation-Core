mod tensor; // Подключаем наш модуль с тензорами
use tensor::SSAUTensor;

fn main() {
    println!("--- Federation Node Genesis ---");

    // 1. Имитируем получение данных о канале связи (например, до соседнего узла B)
    // Параметры: Latency=25ms, Jitter=3ms, Bandwidth=100Mbps, Reliability=0.98, Cost=10
    let link_to_node_b = SSAUTensor::new(25.0, 3.0, 100.0, 0.98, 10.0);

    println!("Узел обнаружил соседа B.");
    
    // 2. ИИ-анализ: Насколько предсказуем этот канал?
    let entropy = link_to_node_b.calculate_entropy();
    println!("Энтропия (неопределенность) канала: {:.4}", entropy);

    // 3. Логика принятия решения (упрощенная)
    if entropy < 0.2 {
        println!("Статус: КАНАЛ СТАБИЛЕН. ИИ подтверждает маршрут.");
    } else {
        println!("Статус: КАНАЛ ШУМНЫЙ. ИИ ищет альтернативные пути.");
    }

    // 4. Демонстрация Проверки Треугольника (Детектор лжи)
    println!("\n--- Проверка верификации (Triangle Check) ---");
    let l_ab = 30.0; // Заявленная задержка между A и B
    let l_ac = 10.0; // Наша задержка до C
    let l_bc = 15.0; // Задержка от C до B (со слов C)

    let is_valid = SSAUTensor::verify_triangle(l_ab, l_ac, l_bc);
    
    if !is_valid {
        println!("ВНИМАНИЕ: Обнаружена аномалия данных! Узел C или B предоставляет ложные метрики.");
        println!("Инициация процесса снижения репутации (Slashing)...");
    } else {
        println!("Данные верифицированы. Геометрия сети в норме.");
    }
}
