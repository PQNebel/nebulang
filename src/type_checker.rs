use super::*;
use Type::*;
use Operator::*;
use Exp::*;

type TypeResult = Result<Type, (String, Location)>;

impl<'a> Exp {
    pub fn type_check(&'a mut self, envir: &'a mut Environment<Type>) -> TypeResult {
        match self {
            BinOpExp(left, op, right, loc) => match op {
                Plus | Minus | Multiply | Divide | Modulo => match (left.type_check(envir)?, right.type_check(envir)?) {
                    (Int, Int) => Ok(Int),
                    (Int, Float) => Ok(Float),
                    (Float, Int) => Ok(Float),
                    (Float, Float) => Ok(Float),
                    (left, right) => Err((format!("Invalid operation {op} for {left} and {right}"), *loc)),
                },
                LessThan | GreaterThan | LessOrEquals | GreaterOrEquals => match (left.type_check(envir)?, right.type_check(envir)?) {
                    (Int, Int) => Ok(Bool),
                    (Int, Float) => Ok(Bool),
                    (Float, Int) => Ok(Bool),
                    (Float, Float) => Ok(Bool),
                    (left, right) => Err((format!("Invalid operation {op} for {left} and {right}"), *loc)),
                },
                Equals | NotEquals => match (left.type_check(envir)?, right.type_check(envir)?) {
                    (Int, Int) => Ok(Bool),
                    (Float, Float) => Ok(Bool),
                    (Bool, Bool) => Ok(Bool),
                    (left, right) => Err((format!("Invalid operation {op} for {left} and {right}"), *loc)),
                },
                And | Or => match (left.type_check(envir)?, right.type_check(envir)?) {
                    (Bool, Bool) => Ok(Bool),
                    (left, right) => Err((format!("Invalid operation {op} for {left} and {right}"), *loc)),
                },
                Assign => match (left.as_ref(), right.type_check(envir)?) {
                    (VarExp(id, loc), value) => {
                        let typ = match envir.lookup_var(id) {
                            Ok(typ) => typ,
                            Err(_) => return Err((format!("Variable {id} does not exist here"), *loc))
                        };
                        if typ != value {
                            Err((format!("Cannot assign {value} to {id} which is {typ}"), *loc))
                        } else {
                            Ok(Unit)
                        }
                    },
                    _ => unreachable!("Not a variable expression")
                },
                PlusAssign | MinusAssign => match (left.as_ref(), right.type_check(envir)?) {
                    (VarExp(id, id_loc), value) => {
                        let typ = match envir.lookup_var(id) {
                            Ok(typ) => typ,
                            Err(_) => return Err((format!("Variable {id} does not exist here"), *id_loc))
                        };
                        match (typ, value) {
                            (Int, Int) => {},
                            (Float, Float) => {},
                            _ => return Err((format!("Cannot add {value} to {id} because it is {typ}"), *loc)),
                        };
                        Ok(Unit)
                    },
                    _ => unreachable!("Not a variable expression")
                }
                Not => unreachable!("Not a binary operator"),
            },
            UnOpExp(op, exp, loc) => match op {
                Minus => match exp.type_check(envir)? {
                    Int => Ok(Int),
                    Float => Ok(Float),
                    typ => Err((format!("Unary operator {op} is not valid for {typ}"), *loc)),
                },
                Not => match exp.type_check(envir)? {
                    Bool => Ok(Bool),
                    typ => Err((format!("Unary operator {op} is not valid for {typ}"), *loc)),
                },
                _ => unreachable!("Not a unary operator")
            },
            LiteralExp(lit, _) => {
                match lit {
                    Literal::Int(_) => Ok(Int),
                    Literal::Float(_) => Ok(Float),
                    Literal::Bool(_) => Ok(Bool),
                    Literal::Unit => unreachable!("Unit should not show up as a literal outside of returns"),
                }
            },
            BlockExp(exps, funs, loc) => {
                envir.enter_scope();

                for i in 0..funs.len() {
                    if envir.fun_exist_in_scope(&funs[i].0) {
                        return Err((format!("Variable '{}' already exist in this scope", funs[i].0), *loc))
                    }
                    envir.push_function(funs[i].0.clone(), funs[i].1.clone());
                }

                envir.update_fun_envirs();

                let mut returned: Type = Unit;
                for exp in exps {
                    returned = exp.type_check(envir)?;
                }

                envir.leave_scope();

                Ok(returned)
            },
            VarExp(id, loc) => {
                match envir.lookup_var(&id) {
                    Ok(typ) => Ok(typ),
                    Err(_) => Err((format!("Variable '{id}' does not exist here"), *loc)),
                }
            },
            LetExp(id, exp, loc) => {
                if envir.var_exist_in_scope(&id) {
                    return Err((format!("Variable '{id}' already exist in this scope"), *loc))
                }
                let value = exp.type_check(envir)?;
                envir.push_variable(id.clone(), value); 
                Ok(Unit)
            },
            IfElseExp(cond, pos, neg, loc) => {
                let cond = cond.type_check(envir)?;
                if cond != Bool {
                    return Err((format!("Condition for if must be boolean, got {cond}"), *loc))
                }
                let pos_type = pos.type_check(envir)?;
                if let Some(neg) = neg {
                    let neg_type = neg.type_check(envir)?;
                    if pos_type != neg_type {
                        return Err((format!("If and else branch must have same type, got {pos_type} and {neg_type}"), *loc))
                    }
                    Ok(pos_type)
                } else {
                    Ok(Unit)
                }
            },
            WhileExp(cond, _, loc) => {
                if cond.type_check(envir)? != Bool {
                    return Err((format!("Condition for while must be boolean, got {cond}"), *loc))
                }
                Ok(Unit)
            }
            FunCallExp(id, args, loc) => {
                let mut closure = match envir.lookup_fun(id) {
                    Ok(clo) => clo,
                    Err(_) => return Err((format!("Function '{id}' does not exist here"), *loc))
                };

                if closure.fun.ret_type == Any {
                    return Err((format!("Recursive function '{id}' need type annotations"), *loc))
                }
                
                if args.len() != closure.fun.param_types.len() {
                    panic!("Incorrect argument count")
                }

                for i in 0..args.len() {
                    if args[i].type_check(envir)? != closure.fun.param_types[i] {
                        panic!("Incorrect argument type")
                    }
                }

                if !closure.declared {
                    let mut renv = envir.get_scope(closure.decl_scope());
                    closure.fun.type_check(&mut renv)?;
                }

                Ok(closure.fun.ret_type)
            },
            FunDeclExp(id, _) => {
                envir.declare_fun(&id);
                let mut clo = envir.lookup_fun(&id).unwrap();
                clo.fun.type_check(&mut clo.envir)
            },
        }
    }
}

impl Function {
    pub fn type_check(&mut self, envir: &mut Environment<Type>) -> TypeResult {
        envir.enter_scope();

        for i in 0..self.param_types.len() {
            envir.push_variable(self.params[i].clone(), self.param_types[i].clone());
        }

        let res = self.exp.type_check(envir)?;
        
        envir.leave_scope();

        if self.ret_type == Any {
            self.ret_type = res
        } else if self.ret_type != res {
            return Err((format!("Return type does not match annotation, got {res} and {} was annotated", self.ret_type), self.loc))
        }

        Ok(res)
    }
}