module lib;

int offset() {
    return 3;
}

public interface IBase {
    int read() {
        return offset();
    }
}

public interface IDerived : IBase {
    int read() {
        return offset() + 4;
    }
}

public int base<T : IBase>(T value) {
    return value.read();
}

public int derived<T : IDerived>(T value) {
    return value.read();
}

public interface IPrivate {
    internal int secret(int value = 3) {
        return value;
    }

    internal int secret(float value) {
        return 7;
    }
}

public struct Private : IPrivate {
}

public int local() {
    Private value;
    return value.secret() + value.secret(1.0);
}
