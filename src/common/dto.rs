#[derive(Debug)]
pub struct PaginatedResponseDto<T> {
    pub data: Vec<T>,
    pub count: usize, // 현재 페이지 데이터 개수
    pub total: i64,   // 전체 데이터 개수
}
