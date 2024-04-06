module.exports = grammar({
    name: 'lambdalisten',

    rules: {
        source_file: $ => $.expression,

        expression: $ => choice(
            $.wrap_expression,
            $.identifier,
            $.literal,
            $.sample,
            $.application_expression,
            $.lambda_expression,
            $.lob_expression,
            $.force_expression,
            $.gen_expression,
            $.let_expression,
            $.annotate_expression,
            $.pair_expression,
            $.unpair_expression,
            $.inl_expression,
            $.inr_expression,
            $.case_expression
        ),

        wrap_expression: $ => seq('(', $.expression, ')'),    

        identifier: $ => /[a-z]+/,

        literal: $ => /\d+/,

        sample: $ => /-?\d+\.\d*/,

        application_expression: $ => prec.left(seq($.expression, $.expression)),

        lambda_expression: $ => prec.right(seq('\\', $.identifier, '.', $.expression)),

        lob_expression: $ => prec.right(seq('&', $.identifier, '.', $.expression)),

        force_expression: $ => prec(2, seq('!', $.expression)),

        gen_expression: $ => prec.right(seq($.expression, '::', $.expression)),

        let_expression: $ => prec.left(-1, seq('let', $.identifier, '=', $.expression, 'in', $.expression)),

        annotate_expression: $ => seq($.expression, ':', $.type),

        pair_expression: $ => seq('(', $.expression, ',', $.expression, ')'),

        unpair_expression: $ => prec.left(-1, seq('let', '(', $.identifier, ',', $.identifier, ')', '=', $.expression, 'in', $.expression)),

        inl_expression: $ => prec(2, seq('inl', $.expression)),

        inr_expression: $ => prec(2, seq('inr', $.expression)),

        case_expression: $ => seq('case', $.expression, '{', 'inl', $.identifier, '=>', $.expression, '|', 'inr', $.identifier, '=>', $.expression, '}'),

        type: $ => choice(
            $.wrap_type,
            $.base_type,
            $.function_type,
            $.stream_type,
            $.product_type,
            $.sum_type
        ),

        wrap_type: $ => seq('(', $.type, ')'),

        base_type: $ => choice(
            'sample',
            'index',
            'unit'
        ),

        function_type: $ => prec.right(seq($.type, '->', $.type)),

        stream_type: $ => prec(3, seq('~', $.type)),

        product_type: $ => prec.right(2, seq($.type, '*', $.type)),

        sum_type: $ => prec.right(1, seq($.type, '+', $.type))
    }
});
