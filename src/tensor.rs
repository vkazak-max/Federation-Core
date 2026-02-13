/// Структура SSAU Тензора (Structural Awareness Unit)
/// Описывает состояние канала связи между двумя узлами.
pub struct SSAUTensor {
    pub latency: f64,      // L (мс)
    pub jitter: f64,       // J (мс)
    pub bandwidth: f64,    // B (Мбит/с)
    pub reliability: f64,  // R (0.0 - 1.0)
    pub energy_cost: f64,  // E (условные единицы)
}

impl SSAUTensor {
    /// Инициализация нового тензора
    pub fn new(l: f64, j: f64, b: f64, r: f64, e: f64) -> Self {
        Self {
            latency: l,
            jitter: j,
            bandwidth: b,
            reliability: r,
            energy_cost: e,
        }
    }

    /// Вычисление Энтропии Пути (H)
    /// Чем выше значение, тем менее предсказуем (хуже) канал.
    pub fn calculate_entropy(&self) -> f64 {
        // Мы используем надежность (R) как вероятность P стабильности канала
        let p = self.reliability.clamp(0.001, 0.999); // Избегаем log(0)
        -(p * p.log2() + (1.0 - p) * (1.0 - p).log2())
    }

    /// Проверка неравенства треугольника (Triangle Check)
    /// Используется для верификации данных от соседей.
    /// |L_ac - L_bc| <= L_ab <= L_ac + L_bc
    pub fn verify_triangle(l_ab: f64, l_ac: f64, l_bc: f64) -> bool {
        let diff = (l_ac - l_bc).abs();
        let sum = l_ac + l_bc;
        l_ab >= diff && l_ab <= sum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy() {
        let tensor = SSAUTensor::new(20.0, 2.0, 100.0, 0.95, 1.0);
        let entropy = tensor.calculate_entropy();
        println!("Entropy: {}", entropy);
        assert!(entropy > 0.0);
    }
}
