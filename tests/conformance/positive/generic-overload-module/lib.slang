module lib;
public int pick<T>(T value) { return 1; }
internal int pick<T, U = int>(T value) { return 2; }
public int resolve<T>(T value) { return pick<T, int>(value); }
