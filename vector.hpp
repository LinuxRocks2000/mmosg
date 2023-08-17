#pragma once


struct vector{
  double x = 0;
  double y = 0;

  vector operator+(vector v){
    return {x + v.x, y + v.y};
  }

  vector operator-(vector v){
    return {x - v.x, y - v.y};
  }

  vector operator-(){
    return flip();
  }

  void operator+=(vector v){
    this -> x += v.x;
    this -> y += v.y;
  }

  void operator-=(vector v){
    this -> x -= v.x;
    this -> y -= v.y;
  }

  double angle(){
    double r = 0;
    if (x != 0){
      r = atan(y/x);
    }
    else{
        if (y > 0){
            r = PI/2; 
        }
        else{
            r = -PI/2;
        }
    }
    if (x < 0){
      r += PI;
    }
    return r;
  }

  float magnitude(){
    return sqrt(x * x + y * y);
  }

  void setMandA(float mag, float ang){
    x = cos(ang) * mag; // Brush up on yer trig young man
    y = sin(ang) * mag;
  }

  void setMagnitude(float mag){
    setMandA(mag, angle());
  }

  void setAngle(float ang){
    setMandA(magnitude(), ang);
  }

  vector rotate(float amount){
    vector r;
    r.setMandA(magnitude(), angle() + amount);
    return r;
  }

  vector flip(){
    vector r;
    r.x = -x;
    r.y = -y;
    return r;
  }

  bool isZero(){
    return (x == 0) && (y == 0);
  }

  void dead(float band){
    if (magnitude() < band){
      zero();
    }
  }

  void zero(){
    x = 0;
    y = 0;
  }

  void cap (float top){
    if (magnitude() > top){
      setMagnitude(top);
    }
  }

  void speedLimit(double cap){
    setMagnitude(magnitude() * cap);
    if (magnitude() > cap){
      setMagnitude(cap);
    }
  }

  void SetPercent(float p){ /* For PIDController compatibility */
    setMagnitude(p);
  }

  std::string string(){
    return "(" + std::to_string(x) + ", " + std::to_string(y) + ")";
  }
};