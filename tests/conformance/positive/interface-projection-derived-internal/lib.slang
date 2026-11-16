module lib;
public interface Base { int read(float input) { return 3; } }
public interface Derived : Base { internal int read(float input) { return 7; } }
public struct Value : Derived {}
public int inside() { Value value; return value.read(1.0); }
