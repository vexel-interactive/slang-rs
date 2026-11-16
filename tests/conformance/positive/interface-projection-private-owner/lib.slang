module lib;
internal float seed() { return 3.9; }
public interface Read { int read(float input = seed()); }
public interface GenericRead<A> { A read(float input); }
public interface StaticRead { static int read(This value, float input); }
