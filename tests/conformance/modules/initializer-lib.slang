module initializer_lib;

float defaultValue() { return 3.0; }
public struct Pair { public float x; float hidden = defaultValue(); };
public struct Container { public Pair pair = {2.0}; public float values[2] = {4.0}; };
public float read(Container value) { return value.pair.hidden + value.values[1]; }
