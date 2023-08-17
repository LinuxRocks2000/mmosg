// 2d trigonometric vectors in JavaScript
// based on vector.rs, vector.hpp, and more
// Rarely the most efficient way to handle trigonometry, but usually the most concise.

class Vector {
    constructor(x, y) {
        this.x = x;
        this.y = y;
    }

    angle() {
        return Math.atan2(this.y, this.x);
    }

    setManda(m, a) {
        this.x = Math.cos(a) * m;
        this.y = Math.sin(a) * m;
    }

    magnitude() {
        return Math.sqrt(this.x * this.x + this.y * this.y);
    }

    setAngle(a) {
        this.setManda(this.magnitude(), a);
    }

    setMagnitude(m) {
        this.setManda(m, this.angle());
    }

    static fromManda(m, a) {
        var vec = new Vector(0, 0);
        vec.setManda(m, a);
        return vec;
    }

    rotate(a) {
        return Vector.fromManda(this.magnitude(), this.angle() + a);
    }

    sort(fun) {
        return new VectorSorter(fun).with(this);
    }
}


class VectorSorter extends Vector{
    constructor(fun) {
        super(0, 0);
        this.matcha = fun;
        this.winner = undefined;
    }

    with(vector) {
        if (this.winner == undefined || this.matcha(vector, this.winner)) {
            this.winner = vector;
            this.x = vector.x;
            this.y = vector.y;
        }
        return this;
    }
}
