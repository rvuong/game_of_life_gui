use game_of_life::sim::Grid;

#[test]
fn grid_stores_dimensions() {
    let g = Grid::new(16, 32);
    assert_eq!(g.width, 16);
    assert_eq!(g.height, 32);
}

#[test]
fn cell_count_is_area() {
    let g = Grid::new(4, 8);
    assert_eq!(g.cell_count(), 32);
}

#[test]
fn unit_grid_has_one_cell() {
    let g = Grid::new(1, 1);
    assert_eq!(g.cell_count(), 1);
}
